use http::request::Parts;
use kartoteka_domain as domain;
use kartoteka_shared::auth_ctx::{UserId, UserLocale};
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

use crate::tools::{
    comments::AddCommentParams,
    items::{CreateItemParams, UpdateItemParams},
    relations::{AddRelationParams, RemoveRelationParams},
    search::SearchItemsParams,
    templates::{CreateListFromTemplateParams, SaveAsTemplateParams},
    time::{LogTimeParams, StartTimerParams},
};
use crate::{McpError, McpI18n};

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
            McpError::BadRequest(r) => ("mcp-err-validation", vec![("reason", r)]),
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
}

#[tool_router]
impl KartotekaServer {
    #[rmcp::tool(name = "create_item", description = "mcp-tool-create_item-desc")]
    async fn create_item(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<CreateItemParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) =
            Self::extract_user_id_and_locale(&parts).map_err(|e| self.map_err(e, "en"))?;
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
            .map_err(|e| self.map_err(McpError::Domain(e), &locale))?;
        self.json_result(item, &locale)
    }

    #[rmcp::tool(name = "update_item", description = "mcp-tool-update_item-desc")]
    async fn update_item(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<UpdateItemParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) =
            Self::extract_user_id_and_locale(&parts).map_err(|e| self.map_err(e, "en"))?;
        let req = domain::items::UpdateItemRequest {
            title: p.title,
            description: p.description,
            completed: p.completed,
            quantity: p.quantity,
            actual_quantity: p.actual_quantity,
            unit: p.unit,
            start_date: p.start_date,
            start_time: p.start_time,
            deadline: p.deadline,
            deadline_time: p.deadline_time,
            hard_deadline: p.hard_deadline,
            estimated_duration: p.estimated_duration,
        };
        let item = domain::items::update(&self.pool, &uid, &p.item_id, &req)
            .await
            .map_err(|e| self.map_err(McpError::Domain(e), &locale))?;
        self.json_result(item, &locale)
    }

    #[rmcp::tool(name = "search_items", description = "mcp-tool-search_items-desc")]
    async fn search_items(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<SearchItemsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) =
            Self::extract_user_id_and_locale(&parts).map_err(|e| self.map_err(e, "en"))?;
        let results = domain::search::search(&self.pool, &uid, &p.query)
            .await
            .map_err(|e| self.map_err(McpError::Domain(e), &locale))?;
        self.json_result(results, &locale)
    }

    #[rmcp::tool(name = "add_comment", description = "mcp-tool-add_comment-desc")]
    async fn add_comment(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<AddCommentParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) =
            Self::extract_user_id_and_locale(&parts).map_err(|e| self.map_err(e, "en"))?;
        let comment = domain::comments::create(
            &self.pool,
            &uid,
            &p.entity_type,
            &p.entity_id,
            &p.content,
            "user",
            None,
        )
        .await
        .map_err(|e| self.map_err(McpError::Domain(e), &locale))?;
        self.json_result(comment, &locale)
    }

    #[rmcp::tool(name = "add_relation", description = "mcp-tool-add_relation-desc")]
    async fn add_relation(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<AddRelationParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) =
            Self::extract_user_id_and_locale(&parts).map_err(|e| self.map_err(e, "en"))?;
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
        .map_err(|e| self.map_err(McpError::Domain(e), &locale))?;
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
        let (uid, locale) =
            Self::extract_user_id_and_locale(&parts).map_err(|e| self.map_err(e, "en"))?;
        domain::relations::delete(&self.pool, &uid, &p.relation_id)
            .await
            .map_err(|e| self.map_err(McpError::Domain(e), &locale))?;
        Ok(CallToolResult::success(vec![
            Content::json(json!({"deleted": true})).expect("json"),
        ]))
    }

    #[rmcp::tool(name = "start_timer", description = "mcp-tool-start_timer-desc")]
    async fn start_timer(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<StartTimerParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) =
            Self::extract_user_id_and_locale(&parts).map_err(|e| self.map_err(e, "en"))?;
        let entry = domain::time_entries::start(&self.pool, &uid, p.item_id.as_deref())
            .await
            .map_err(|e| self.map_err(McpError::Domain(e), &locale))?;
        self.json_result(entry, &locale)
    }

    #[rmcp::tool(name = "stop_timer", description = "mcp-tool-stop_timer-desc")]
    async fn stop_timer(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) =
            Self::extract_user_id_and_locale(&parts).map_err(|e| self.map_err(e, "en"))?;
        let entry = domain::time_entries::stop(&self.pool, &uid)
            .await
            .map_err(|e| self.map_err(McpError::Domain(e), &locale))?;
        self.json_result(entry, &locale)
    }

    #[rmcp::tool(name = "log_time", description = "mcp-tool-log_time-desc")]
    async fn log_time(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<LogTimeParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) =
            Self::extract_user_id_and_locale(&parts).map_err(|e| self.map_err(e, "en"))?;
        let entry = domain::time_entries::log_manual(
            &self.pool,
            &uid,
            p.item_id.as_deref(),
            &p.started_at,
            &p.ended_at,
            p.description.as_deref(),
        )
        .await
        .map_err(|e| self.map_err(McpError::Domain(e), &locale))?;
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
        let (uid, locale) =
            Self::extract_user_id_and_locale(&parts).map_err(|e| self.map_err(e, "en"))?;
        let list_type = p.list_type.as_deref().unwrap_or("custom");
        let list = domain::templates::create_list_from_template(
            &self.pool,
            &uid,
            &p.template_id,
            &p.list_name,
            list_type,
        )
        .await
        .map_err(|e| self.map_err(McpError::Domain(e), &locale))?;
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
        let (uid, locale) =
            Self::extract_user_id_and_locale(&parts).map_err(|e| self.map_err(e, "en"))?;
        let tmpl =
            domain::templates::create_from_list(&self.pool, &uid, &p.list_id, &p.template_name)
                .await
                .map_err(|e| self.map_err(McpError::Domain(e), &locale))?;
        self.json_result(tmpl, &locale)
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
        use kartoteka_shared::auth_ctx::UserId;

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

        let json = match parsed {
            ResourceUri::Lists => {
                let data = kartoteka_domain::lists::list_all(&self.pool, &user_id)
                    .await
                    .map_err(|e| self.map_err(McpError::Domain(e), &locale))?;
                serde_json::to_value(data)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            ResourceUri::ListDetail(id) => {
                let data = kartoteka_domain::lists::get_one(&self.pool, &id, &user_id)
                    .await
                    .map_err(|e| self.map_err(McpError::Domain(e), &locale))?
                    .ok_or_else(|| {
                        ErrorData::invalid_params(
                            self.i18n.translate_args(
                                &locale,
                                "mcp-err-not-found",
                                &[("entity", "list")],
                            ),
                            None,
                        )
                    })?;
                serde_json::to_value(data)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            ResourceUri::ListItems { list_id, .. } => {
                let data = kartoteka_domain::items::list_for_list(&self.pool, &user_id, &list_id)
                    .await
                    .map_err(|e| self.map_err(McpError::Domain(e), &locale))?;
                serde_json::to_value(data)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            ResourceUri::Containers => {
                let data = kartoteka_domain::containers::list_all(&self.pool, &user_id)
                    .await
                    .map_err(|e| self.map_err(McpError::Domain(e), &locale))?;
                serde_json::to_value(data)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            ResourceUri::ContainerDetail(id) => {
                let data = kartoteka_domain::containers::get_one(&self.pool, &id, &user_id)
                    .await
                    .map_err(|e| self.map_err(McpError::Domain(e), &locale))?;
                serde_json::to_value(data)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            ResourceUri::Tags { .. } => {
                let data = kartoteka_domain::tags::list_all(&self.pool, &user_id)
                    .await
                    .map_err(|e| self.map_err(McpError::Domain(e), &locale))?;
                serde_json::to_value(data)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            ResourceUri::Today => {
                let data = kartoteka_domain::items::by_date(&self.pool, &user_id, "today")
                    .await
                    .map_err(|e| self.map_err(McpError::Domain(e), &locale))?;
                serde_json::to_value(data)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
            }
            ResourceUri::TimeSummary => {
                let data = kartoteka_domain::time_entries::list_all_for_user(&self.pool, &user_id)
                    .await
                    .map_err(|e| self.map_err(McpError::Domain(e), &locale))?;
                serde_json::to_value(data)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
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
