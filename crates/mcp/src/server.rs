use http::request::Parts;
use kartoteka_db::{self as db, lists::InsertListInput};
use kartoteka_domain as domain;
use kartoteka_shared::{
    auth_ctx::{UserId, UserLocale},
    types::CreateContainerRequest,
};
use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    handler::server::{
        router::tool::ToolRouter,
        tool::{Extension, Parameters, ToolCallContext},
    },
    model::{
        CallToolRequestParam, CallToolResult, Content, ListResourceTemplatesResult,
        ListResourcesResult, ListToolsResult, PaginatedRequestParam, ReadResourceRequestParam,
        ReadResourceResult, ResourceContents, ServerCapabilities, ServerInfo,
    },
    service::RequestContext,
    tool_router,
};
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

mod annotations;
mod batch;

use crate::client_ref::RefResolver;
use crate::tools::{
    comments::AddCommentParams,
    items::{
        CreateContainerParams, CreateContainersParams, CreateItemParams, CreateItemsParams,
        CreateListParams, CreateListsParams, UpdateItemParams,
    },
    read::{GetContainerParams, GetItemParams, GetListParams, ListItemsParams},
    relations::{AddRelationParams, RemoveRelationParams},
    search::SearchItemsParams,
    templates::{CreateListFromTemplateParams, SaveAsTemplateParams},
    time::{LogTimeParams, StartTimerParams},
};
use crate::{McpError, McpI18n};
use batch::PositionAllocator;

pub struct KartotekaServer {
    pub(crate) pool: SqlitePool,
    pub(crate) i18n: Arc<McpI18n>,
    pub(crate) tool_router: ToolRouter<Self>,
}

impl Clone for KartotekaServer {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            i18n: self.i18n.clone(),
            tool_router: Self::tool_router(),
        }
    }
}

impl KartotekaServer {
    pub fn new(pool: SqlitePool, i18n: Arc<McpI18n>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            pool,
            i18n,
        }
    }

    pub(crate) fn map_err(&self, e: McpError, locale: &str) -> ErrorData {
        let (key, args): (&str, Vec<(&str, &str)>) = match &e {
            McpError::Unauthorized => ("mcp-err-unauthorized", vec![]),
            McpError::Domain(domain::DomainError::NotFound(k)) => {
                ("mcp-err-not-found", vec![("entity", k)])
            }
            McpError::Domain(domain::DomainError::Validation(r)) => {
                ("mcp-err-validation", vec![("reason", r)])
            }
            McpError::Domain(domain::DomainError::FeatureRequired(f)) => {
                ("mcp-err-feature-required", vec![("feature", f)])
            }
            McpError::Domain(domain::DomainError::Forbidden) => ("mcp-err-forbidden", vec![]),
            McpError::BadRequest(r) => ("mcp-err-validation", vec![("reason", r.as_str())]),
            _ => ("mcp-err-internal", vec![]),
        };
        let msg = self.i18n.translate_args(locale, key, &args);
        ErrorData::invalid_request(msg, None)
    }

    fn extract_user_id_and_locale(parts: &Parts) -> Result<(String, String), McpError> {
        let user_id = parts
            .extensions
            .get::<UserId>()
            .map(|u| u.0.clone())
            .ok_or(McpError::Unauthorized)?;
        let locale = parts
            .extensions
            .get::<UserLocale>()
            .map(|l| l.0.clone())
            .unwrap_or_else(|| "en".to_string());
        Ok((user_id, locale))
    }

    /// Pull `(user_id, locale)` out of request extensions, mapping any failure
    /// straight to a localized `ErrorData` so handlers can `?` it directly.
    fn auth(&self, parts: &Parts) -> Result<(String, String), ErrorData> {
        Self::extract_user_id_and_locale(parts).map_err(|e| self.map_err(e, "en"))
    }

    /// Closure factory that maps a `DomainError` to localized `ErrorData`.
    /// Pass directly to `.map_err(...)` to avoid the noisy
    /// `|e| self.map_err(McpError::Domain(e), &locale)` boilerplate.
    fn domain_err<'a>(&'a self, locale: &'a str) -> impl Fn(domain::DomainError) -> ErrorData + 'a {
        move |e| self.map_err(McpError::Domain(e), locale)
    }

    /// Same idea for `db::DbError` — wraps via `Into<DomainError>`.
    fn db_err<'a>(&'a self, locale: &'a str) -> impl Fn(db::DbError) -> ErrorData + 'a {
        move |e| self.map_err(McpError::Domain(e.into()), locale)
    }

    /// Same idea for sqlx errors that occur outside `db::*` (begin/commit).
    fn sqlx_err<'a>(&'a self, locale: &'a str) -> impl Fn(sqlx::Error) -> ErrorData + 'a {
        move |e| self.map_err(McpError::Domain(db::DbError::Sqlx(e).into()), locale)
    }

    fn json_result<T: serde::Serialize>(
        &self,
        value: T,
        locale: &str,
    ) -> Result<CallToolResult, ErrorData> {
        let v =
            serde_json::to_value(value).map_err(|e| self.map_err(McpError::Serde(e), locale))?;
        Ok(CallToolResult::success(vec![Content::json(v).map_err(
            |e| ErrorData::invalid_request(e.to_string(), None),
        )?]))
    }

    /// Verify every id in `ids` belongs to `uid`. `kind_label` appears in the
    /// error message so callers can distinguish container vs parent-container
    /// references.
    async fn ensure_containers_owned(
        &self,
        uid: &str,
        ids: &[&str],
        kind_label: &str,
        locale: &str,
    ) -> Result<(), ErrorData> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut unique: Vec<&str> = ids.to_vec();
        unique.sort_unstable();
        unique.dedup();
        let owned = db::containers::find_owned_ids(&self.pool, uid, &unique)
            .await
            .map_err(self.db_err(locale))?;
        if unique.iter().any(|id| !owned.contains(*id)) {
            return Err(self.map_err(
                McpError::BadRequest(format!("one or more {kind_label} values not found")),
                locale,
            ));
        }
        Ok(())
    }
}

#[tool_router]
impl KartotekaServer {
    #[rmcp::tool(name = "create_item", description = "mcp-tool-create_item-desc")]
    async fn create_item(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<CreateItemParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let req = domain::items::CreateItemRequest {
            title: p.title,
            description: p.description,
            start_date: p.start_date,
            deadline: p.deadline,
            hard_deadline: p.hard_deadline,
            start_time: p.start_time,
            deadline_time: p.deadline_time,
            quantity: p.quantity,
            actual_quantity: p.actual_quantity,
            unit: p.unit,
            estimated_duration: p.estimated_duration,
        };
        let item = domain::items::create(&self.pool, &uid, &p.list_id, &req)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(item, &locale)
    }

    #[rmcp::tool(name = "update_item", description = "mcp-tool-update_item-desc")]
    async fn update_item(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<UpdateItemParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let req = domain::items::UpdateItemRequest {
            title: p.title.clone(),
            description: p.description_field(),
            completed: p.completed,
            quantity: p.quantity_field(),
            actual_quantity: p.actual_quantity_field(),
            unit: p.unit_field(),
            start_date: p.start_date_field(),
            start_time: p.start_time_field(),
            deadline: p.deadline_field(),
            deadline_time: p.deadline_time_field(),
            hard_deadline: p.hard_deadline_field(),
            estimated_duration: p.estimated_duration_field(),
        };
        let item = domain::items::update(&self.pool, &uid, &p.item_id, &req)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(item, &locale)
    }

    #[rmcp::tool(name = "create_list", description = "mcp-tool-create_list-desc")]
    async fn create_list(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<CreateListParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let req = domain::lists::CreateListRequest {
            name: p.name,
            list_type: p.list_type,
            icon: p.icon,
            description: p.description,
            container_id: p.container_id,
            parent_list_id: p.parent_list_id,
            features: p.features.unwrap_or_default(),
        };
        let list = domain::lists::create(&self.pool, &uid, &req)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(list, &locale)
    }

    #[rmcp::tool(name = "search_items", description = "mcp-tool-search_items-desc")]
    async fn search_items(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<SearchItemsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let results = domain::search::search(&self.pool, &uid, &p.query)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(results, &locale)
    }

    #[rmcp::tool(name = "add_comment", description = "mcp-tool-add_comment-desc")]
    async fn add_comment(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<AddCommentParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let comment = domain::comments::create(
            &self.pool,
            &uid,
            &p.entity_type,
            &p.entity_id,
            &p.content,
            "user",
            p.author_name.as_deref(),
        )
        .await
        .map_err(self.domain_err(&locale))?;
        self.json_result(comment, &locale)
    }

    #[rmcp::tool(name = "add_relation", description = "mcp-tool-add_relation-desc")]
    async fn add_relation(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<AddRelationParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let rel = domain::relations::create(
            &self.pool,
            &uid,
            &p.from_type,
            &p.from_id,
            &p.to_type,
            &p.to_id,
            &p.relation_type,
        )
        .await
        .map_err(self.domain_err(&locale))?;
        self.json_result(rel, &locale)
    }

    #[rmcp::tool(
        name = "remove_relation",
        description = "mcp-tool-remove_relation-desc"
    )]
    async fn remove_relation(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<RemoveRelationParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        domain::relations::delete(&self.pool, &uid, &p.relation_id)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(json!({"deleted": true}), &locale)
    }

    #[rmcp::tool(name = "start_timer", description = "mcp-tool-start_timer-desc")]
    async fn start_timer(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<StartTimerParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let entry = domain::time_entries::start(&self.pool, &uid, p.item_id.as_deref())
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(entry, &locale)
    }

    #[rmcp::tool(name = "stop_timer", description = "mcp-tool-stop_timer-desc")]
    async fn stop_timer(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let entry = domain::time_entries::stop(&self.pool, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(entry, &locale)
    }

    #[rmcp::tool(name = "log_time", description = "mcp-tool-log_time-desc")]
    async fn log_time(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<LogTimeParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let entry = domain::time_entries::log_manual(
            &self.pool,
            &uid,
            p.item_id.as_deref(),
            &p.started_at,
            &p.ended_at,
            p.description.as_deref(),
        )
        .await
        .map_err(self.domain_err(&locale))?;
        self.json_result(entry, &locale)
    }

    #[rmcp::tool(
        name = "create_list_from_template",
        description = "mcp-tool-create_list_from_template-desc"
    )]
    async fn create_list_from_template(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<CreateListFromTemplateParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let list_type = p.list_type.as_deref().unwrap_or("custom");
        let list = domain::templates::create_list_from_template(
            &self.pool,
            &uid,
            &p.template_id,
            &p.list_name,
            list_type,
        )
        .await
        .map_err(self.domain_err(&locale))?;
        self.json_result(list, &locale)
    }

    #[rmcp::tool(
        name = "save_as_template",
        description = "mcp-tool-save_as_template-desc"
    )]
    async fn save_as_template(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<SaveAsTemplateParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let tmpl =
            domain::templates::create_from_list(&self.pool, &uid, &p.list_id, &p.template_name)
                .await
                .map_err(self.domain_err(&locale))?;
        self.json_result(tmpl, &locale)
    }

    // ── Read-only resource-compat tools ──────────────────────────────────────

    #[rmcp::tool(name = "list_lists", description = "mcp-tool-list_lists-desc")]
    async fn list_lists(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let data = domain::lists::list_all(&self.pool, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(data, &locale)
    }

    #[rmcp::tool(name = "get_list", description = "mcp-tool-get_list-desc")]
    async fn get_list(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<GetListParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let data = domain::lists::get_one(&self.pool, &p.list_id, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(data, &locale)
    }

    #[rmcp::tool(name = "list_items", description = "mcp-tool-list_items-desc")]
    async fn list_items(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<ListItemsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let all = domain::items::list_for_list(&self.pool, &p.list_id, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        let limit = domain::paging::clamp_limit(p.limit);
        let offset: usize = p
            .cursor
            .as_deref()
            .and_then(|c| c.parse().ok())
            .unwrap_or(0);
        let page: Vec<_> = all
            .iter()
            .skip(offset)
            .take(limit as usize)
            .cloned()
            .collect();
        let next_cursor = if offset + page.len() < all.len() {
            Some((offset + page.len()).to_string())
        } else {
            None
        };
        self.json_result(
            domain::paging::Paged {
                data: page,
                next_cursor,
                limit,
            },
            &locale,
        )
    }

    #[rmcp::tool(
        name = "list_containers",
        description = "mcp-tool-list_containers-desc"
    )]
    async fn list_containers(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let data = domain::containers::list_all(&self.pool, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(data, &locale)
    }

    #[rmcp::tool(name = "get_container", description = "mcp-tool-get_container-desc")]
    async fn get_container(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<GetContainerParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let data = domain::containers::get_one(&self.pool, &p.container_id, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(data, &locale)
    }

    #[rmcp::tool(name = "list_tags", description = "mcp-tool-list_tags-desc")]
    async fn list_tags(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let data = domain::tags::list_all(&self.pool, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(data, &locale)
    }

    #[rmcp::tool(name = "get_today", description = "mcp-tool-get_today-desc")]
    async fn get_today(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let data = domain::items::by_date(&self.pool, &uid, "today")
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(data, &locale)
    }

    #[rmcp::tool(
        name = "get_time_summary",
        description = "mcp-tool-get_time_summary-desc"
    )]
    async fn get_time_summary(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let data = domain::time_entries::list_all_for_user(&self.pool, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(data, &locale)
    }

    #[rmcp::tool(name = "get_item", description = "mcp-tool-get_item-desc")]
    async fn get_item(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<GetItemParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let data = domain::items::get_one(&self.pool, &p.item_id, &uid)
            .await
            .map_err(self.domain_err(&locale))?
            .ok_or_else(|| {
                self.map_err(
                    McpError::Domain(domain::DomainError::NotFound("item")),
                    &locale,
                )
            })?;
        self.json_result(data, &locale)
    }

    #[rmcp::tool(name = "list_templates", description = "mcp-tool-list_templates-desc")]
    async fn list_templates(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let data = domain::templates::list(&self.pool, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(data, &locale)
    }

    #[rmcp::tool(name = "list_overdue", description = "mcp-tool-list_overdue-desc")]
    async fn list_overdue(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let data = domain::items::overdue(&self.pool, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(data, &locale)
    }

    #[rmcp::tool(
        name = "get_active_timer",
        description = "mcp-tool-get_active_timer-desc"
    )]
    async fn get_active_timer(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let data = domain::time_entries::get_active(&self.pool, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(data, &locale)
    }

    // ── New tools: create_container + batch creates ───────────────────────────

    #[rmcp::tool(
        name = "create_container",
        description = "mcp-tool-create_container-desc"
    )]
    #[tracing::instrument(skip(self, parts), fields(action = "mcp_create_container"))]
    async fn create_container(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<CreateContainerParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let req = CreateContainerRequest {
            name: p.name,
            icon: p.icon,
            description: p.description,
            status: p.status,
            parent_container_id: p.parent_container_id,
        };
        let container = domain::containers::create(&self.pool, &uid, &req)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(container, &locale)
    }

    #[rmcp::tool(name = "create_items", description = "mcp-tool-create_items-desc")]
    #[tracing::instrument(skip(self, parts), fields(action = "mcp_create_items"))]
    async fn create_items(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<CreateItemsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;

        if p.items.is_empty() {
            return self.json_result(Vec::<serde_json::Value>::new(), &locale);
        }

        // Validate list ownership + get starting position in one query
        let ctx = db::lists::get_create_item_context(&self.pool, &p.list_id, &uid)
            .await
            .map_err(self.db_err(&locale))?
            .ok_or_else(|| {
                self.map_err(
                    McpError::Domain(domain::DomainError::NotFound("list")),
                    &locale,
                )
            })?;

        let start_pos = ctx.next_position as i32;
        let inputs: Vec<db::items::InsertItemInput> = p
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| db::items::InsertItemInput {
                id: Uuid::new_v4().to_string(),
                list_id: p.list_id.clone(),
                position: start_pos + i as i32,
                title: item.title.clone(),
                description: item.description.clone(),
                start_date: item.start_date.clone(),
                deadline: item.deadline.clone(),
                hard_deadline: item.hard_deadline.clone(),
                start_time: item.start_time.clone(),
                deadline_time: item.deadline_time.clone(),
                quantity: item.quantity,
                actual_quantity: item.actual_quantity,
                unit: item.unit.clone(),
                estimated_duration: item.estimated_duration,
            })
            .collect();

        let mut tx = self.pool.begin().await.map_err(self.sqlx_err(&locale))?;
        db::items::insert_many_in_tx(&mut tx, &inputs)
            .await
            .map_err(self.db_err(&locale))?;
        tx.commit().await.map_err(self.sqlx_err(&locale))?;

        let result: Vec<_> = inputs
            .iter()
            .map(|i| serde_json::json!({"id": i.id, "title": i.title}))
            .collect();
        self.json_result(result, &locale)
    }

    #[rmcp::tool(name = "create_lists", description = "mcp-tool-create_lists-desc")]
    #[tracing::instrument(skip(self, parts), fields(action = "mcp_create_lists"))]
    async fn create_lists(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<CreateListsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;

        if p.lists.is_empty() {
            return self.json_result(Vec::<serde_json::Value>::new(), &locale);
        }

        let container_ids: Vec<&str> = p
            .lists
            .iter()
            .filter_map(|l| l.container_id.as_deref())
            .collect();
        self.ensure_containers_owned(&uid, &container_ids, "container_id", &locale)
            .await?;

        // Pre-fetch next_position per (container_id, parent_list_id) scope so
        // batch creates don't collide with existing rows. Entries that resolve
        // their parent via *_ref point at brand-new rows whose default base 0
        // is correct.
        let mut positions: PositionAllocator<(Option<String>, Option<String>)> =
            PositionAllocator::new();
        let mut needed_scopes: Vec<(Option<String>, Option<String>)> = Vec::new();
        for l in &p.lists {
            if l.container_ref.is_none() && l.parent_list_ref.is_none() {
                let key = (l.container_id.clone(), l.parent_list_id.clone());
                if !needed_scopes.contains(&key) {
                    needed_scopes.push(key);
                }
            }
        }
        for key in &needed_scopes {
            let base =
                db::lists::next_position(&self.pool, &uid, key.0.as_deref(), key.1.as_deref())
                    .await
                    .map_err(self.db_err(&locale))?;
            positions.set_base(key.clone(), base);
        }

        let mut tx = self.pool.begin().await.map_err(self.sqlx_err(&locale))?;
        let mut resolver = RefResolver::new();
        let mut result = Vec::with_capacity(p.lists.len());

        for list in &p.lists {
            let container_id = resolver
                .pick(
                    list.container_id.as_deref(),
                    list.container_ref.as_deref(),
                    false,
                )
                .map_err(|e| self.map_err(McpError::BadRequest(e.to_string()), &locale))?;
            let parent_list_id = resolver
                .pick(
                    list.parent_list_id.as_deref(),
                    list.parent_list_ref.as_deref(),
                    false,
                )
                .map_err(|e| self.map_err(McpError::BadRequest(e.to_string()), &locale))?;

            let position = positions.allocate((
                container_id.map(str::to_owned),
                parent_list_id.map(str::to_owned),
            ));

            let list_type = list.list_type.as_deref().unwrap_or("custom");
            let new_id = Uuid::new_v4().to_string();

            db::lists::insert(
                &mut tx,
                &InsertListInput {
                    id: new_id.clone(),
                    user_id: uid.clone(),
                    position,
                    name: list.name.clone(),
                    icon: list.icon.clone(),
                    description: list.description.clone(),
                    list_type: list_type.to_owned(),
                    container_id: container_id.map(str::to_owned),
                    parent_list_id: parent_list_id.map(str::to_owned),
                },
            )
            .await
            .map_err(self.db_err(&locale))?;

            if let Some(features) = &list.features {
                if !features.is_empty() {
                    let features_json = domain::lists::features_from_names(features);
                    db::lists::set_features(&mut tx, &new_id, &features_json)
                        .await
                        .map_err(self.db_err(&locale))?;
                }
            }

            resolver
                .register(list.client_ref.as_deref(), &new_id)
                .map_err(|e| self.map_err(McpError::BadRequest(e.to_string()), &locale))?;

            result.push(serde_json::json!({"id": new_id, "name": list.name}));
        }

        tx.commit().await.map_err(self.sqlx_err(&locale))?;
        self.json_result(result, &locale)
    }

    #[rmcp::tool(
        name = "create_containers",
        description = "mcp-tool-create_containers-desc"
    )]
    #[tracing::instrument(skip(self, parts), fields(action = "mcp_create_containers"))]
    async fn create_containers(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<CreateContainersParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;

        if p.containers.is_empty() {
            return self.json_result(Vec::<serde_json::Value>::new(), &locale);
        }

        let parent_ids: Vec<&str> = p
            .containers
            .iter()
            .filter_map(|c| c.parent_container_id.as_deref())
            .collect();
        self.ensure_containers_owned(&uid, &parent_ids, "parent_container_id", &locale)
            .await?;

        for c in &p.containers {
            domain::rules::containers::validate_status(c.status.as_deref())
                .map_err(self.domain_err(&locale))?;
        }

        // Pre-fetch next_position per parent scope.
        let mut positions: PositionAllocator<Option<String>> = PositionAllocator::new();
        let mut needed_scopes: Vec<Option<String>> = Vec::new();
        for c in &p.containers {
            if c.parent_container_ref.is_none() && !needed_scopes.contains(&c.parent_container_id) {
                needed_scopes.push(c.parent_container_id.clone());
            }
        }
        for key in &needed_scopes {
            let base = db::containers::next_position(&self.pool, &uid, key.as_deref())
                .await
                .map_err(self.db_err(&locale))?;
            positions.set_base(key.clone(), base as i64);
        }

        let mut tx = self.pool.begin().await.map_err(self.sqlx_err(&locale))?;
        let mut resolver = RefResolver::new();
        let mut result = Vec::with_capacity(p.containers.len());

        for container in &p.containers {
            let parent_id = resolver
                .pick(
                    container.parent_container_id.as_deref(),
                    container.parent_container_ref.as_deref(),
                    false,
                )
                .map_err(|e| self.map_err(McpError::BadRequest(e.to_string()), &locale))?;

            let position = positions.allocate(parent_id.map(str::to_owned)) as i32;

            let new_id = Uuid::new_v4().to_string();
            let req = CreateContainerRequest {
                name: container.name.clone(),
                icon: container.icon.clone(),
                description: container.description.clone(),
                status: container.status.clone(),
                parent_container_id: parent_id.map(str::to_owned),
            };

            db::containers::insert_in_tx(&mut tx, &new_id, &uid, &req, position)
                .await
                .map_err(self.db_err(&locale))?;

            resolver
                .register(container.client_ref.as_deref(), &new_id)
                .map_err(|e| self.map_err(McpError::BadRequest(e.to_string()), &locale))?;

            result.push(serde_json::json!({"id": new_id, "name": container.name}));
        }

        tx.commit().await.map_err(self.sqlx_err(&locale))?;
        self.json_result(result, &locale)
    }
}

// ── ServerHandler impl ────────────────────────────────────────────────────────

impl ServerHandler for KartotekaServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: rmcp::model::Implementation {
                name: "kartoteka".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            instructions: Some(
                "Kartoteka is a personal task and list manager. \
                Start every session by calling list_lists to orient yourself — \
                you need list IDs for most item operations. \
                Use search_items to find items by keyword before acting on them. \
                When adding comments: omit author_name to write in the user's voice; \
                set author_name to your name (e.g. \"Claude\") for your own observations. \
                Prefer get_today and list_overdue to surface what needs attention now."
                    .into(),
            ),
            ..ServerInfo::default()
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let locale = context
            .extensions
            .get::<UserLocale>()
            .map(|l| l.0.clone())
            .unwrap_or_else(|| "en".to_string());
        let mut tools = self.tool_router.list_all();
        for t in &mut tools {
            if let Some(key) = t.description.as_deref() {
                let translated = self.i18n.translate(&locale, key);
                t.description = Some(translated.into());
            }
            t.annotations = Some(annotations::for_tool(t.name.as_ref()));
        }
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let tcc = ToolCallContext::new(self, request, context);
        self.tool_router.call(tcc).await
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let locale = context
            .extensions
            .get::<UserLocale>()
            .map(|l| l.0.clone())
            .unwrap_or_else(|| "en".to_string());
        Ok(ListResourcesResult {
            resources: crate::resources::static_resources(&self.i18n, &locale),
            next_cursor: None,
        })
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        let locale = context
            .extensions
            .get::<UserLocale>()
            .map(|l| l.0.clone())
            .unwrap_or_else(|| "en".to_string());
        Ok(ListResourceTemplatesResult {
            resource_templates: crate::resources::resource_templates(&self.i18n, &locale),
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, ErrorData> {
        use crate::resources::{ResourceUri, parse as parse_uri};

        let user_id = context
            .extensions
            .get::<UserId>()
            .map(|u| u.0.clone())
            .ok_or_else(|| ErrorData::invalid_request("unauthorized", None))?;
        let locale = context
            .extensions
            .get::<UserLocale>()
            .map(|l| l.0.clone())
            .unwrap_or_else(|| "en".to_string());

        let parsed = parse_uri(&request.uri).map_err(|_| {
            ErrorData::invalid_params(
                self.i18n
                    .translate_args(&locale, "mcp-err-bad-uri", &[("uri", &request.uri)]),
                None,
            )
        })?;

        let to_internal = |e: serde_json::Error| ErrorData::internal_error(e.to_string(), None);

        let json = match parsed {
            ResourceUri::Lists => {
                let data = domain::lists::list_all(&self.pool, &user_id)
                    .await
                    .map_err(self.domain_err(&locale))?;
                serde_json::to_value(data).map_err(to_internal)?
            }
            ResourceUri::ListDetail(id) => {
                let data = domain::lists::get_one(&self.pool, &id, &user_id)
                    .await
                    .map_err(self.domain_err(&locale))?
                    .ok_or_else(|| {
                        self.map_err(
                            McpError::Domain(domain::DomainError::NotFound("list")),
                            &locale,
                        )
                    })?;
                serde_json::to_value(data).map_err(to_internal)?
            }
            ResourceUri::ListItems { list_id, .. } => {
                let data = domain::items::list_for_list(&self.pool, &list_id, &user_id)
                    .await
                    .map_err(self.domain_err(&locale))?;
                serde_json::to_value(data).map_err(to_internal)?
            }
            ResourceUri::Containers => {
                let data = domain::containers::list_all(&self.pool, &user_id)
                    .await
                    .map_err(self.domain_err(&locale))?;
                serde_json::to_value(data).map_err(to_internal)?
            }
            ResourceUri::ContainerDetail(id) => {
                let data = domain::containers::get_one(&self.pool, &id, &user_id)
                    .await
                    .map_err(self.domain_err(&locale))?;
                serde_json::to_value(data).map_err(to_internal)?
            }
            ResourceUri::Tags { .. } => {
                let data = domain::tags::list_all(&self.pool, &user_id)
                    .await
                    .map_err(self.domain_err(&locale))?;
                serde_json::to_value(data).map_err(to_internal)?
            }
            ResourceUri::Today => {
                let data = domain::items::by_date(&self.pool, &user_id, "today")
                    .await
                    .map_err(self.domain_err(&locale))?;
                serde_json::to_value(data).map_err(to_internal)?
            }
            ResourceUri::TimeSummary => {
                let data = domain::time_entries::list_all_for_user(&self.pool, &user_id)
                    .await
                    .map_err(self.domain_err(&locale))?;
                serde_json::to_value(data).map_err(to_internal)?
            }
        };

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(
                serde_json::to_string_pretty(&json).unwrap_or_default(),
                request.uri,
            )],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts_with_extensions(extensions: http::Extensions) -> Parts {
        let mut req = http::Request::new(());
        *req.extensions_mut() = extensions;
        let (parts, _) = req.into_parts();
        parts
    }

    #[test]
    fn extract_returns_user_and_locale_when_present() {
        let mut ext = http::Extensions::new();
        ext.insert(UserId("alice".into()));
        ext.insert(UserLocale("pl".into()));
        let parts = parts_with_extensions(ext);

        let (uid, locale) = KartotekaServer::extract_user_id_and_locale(&parts).unwrap();
        assert_eq!(uid, "alice");
        assert_eq!(locale, "pl");
    }

    #[test]
    fn extract_falls_back_to_en_when_locale_missing() {
        let mut ext = http::Extensions::new();
        ext.insert(UserId("bob".into()));
        let parts = parts_with_extensions(ext);

        let (uid, locale) = KartotekaServer::extract_user_id_and_locale(&parts).unwrap();
        assert_eq!(uid, "bob");
        assert_eq!(locale, "en");
    }

    #[test]
    fn extract_errors_when_user_id_missing() {
        let parts = parts_with_extensions(http::Extensions::new());
        let err = KartotekaServer::extract_user_id_and_locale(&parts).unwrap_err();
        assert!(matches!(err, McpError::Unauthorized));
    }
}
