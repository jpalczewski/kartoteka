use http::request::Parts;
use kartoteka_db as db;
use kartoteka_db::lists::InsertListInput;
use kartoteka_domain as domain;
use rmcp::{
    ErrorData,
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

use crate::McpError;
use crate::client_ref::RefResolver;
use crate::server::{KartotekaServer, batch::PositionAllocator};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateListParams {
    pub name: String,
    /// List type: "checklist" (default), "shopping", "schedule", "log", "notes"
    pub list_type: Option<String>,
    pub icon: Option<String>,
    /// Accent colour as `#rrggbb` (null = neutral).
    pub color: Option<String>,
    pub description: Option<String>,
    pub container_id: Option<String>,
    pub parent_list_id: Option<String>,
    /// Feature names to enable, e.g. ["quantity", "deadline"]
    pub features: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateListsInput {
    pub name: String,
    /// List type: "checklist" (default), "shopping", "schedule", "log", "notes"
    pub list_type: Option<String>,
    pub icon: Option<String>,
    /// Accent colour as `#rrggbb` (null = neutral).
    pub color: Option<String>,
    pub description: Option<String>,
    pub features: Option<Vec<String>>,
    /// Real UUID of an existing container. Use `container_ref` instead to
    /// reference a container created earlier in this batch.
    pub container_id: Option<String>,
    /// `client_ref` of a container created earlier in this same batch.
    pub container_ref: Option<String>,
    /// Real UUID of an existing parent list.
    pub parent_list_id: Option<String>,
    /// `client_ref` of a list created earlier in this same batch.
    pub parent_list_ref: Option<String>,
    /// Label that other entries in this batch can reference via `*_ref` fields.
    pub client_ref: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateListsParams {
    pub lists: Vec<CreateListsInput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetListParams {
    pub list_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveListToContainerParams {
    pub list_id: String,
    /// ID of the container to move the list into. Pass null to detach the list from its current container.
    pub container_id: Option<String>,
}

#[tool_router(router = lists_router, vis = "pub(crate)")]
impl KartotekaServer {
    #[rmcp::tool(name = "create_list", description = "mcp-tool-create_list-desc")]
    pub(crate) async fn create_list(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<CreateListParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let req = domain::lists::CreateListRequest {
            name: p.name,
            list_type: p.list_type,
            icon: p.icon,
            color: p.color,
            description: p.description,
            container_id: p.container_id,
            parent_list_id: p.parent_list_id,
            features: p.features.unwrap_or_default(),
        };
        self.domain_or_existing(domain::lists::create(&self.pool, &uid, &req).await, &locale)
    }

    #[rmcp::tool(name = "create_lists", description = "mcp-tool-create_lists-desc")]
    #[tracing::instrument(skip(self, parts), fields(action = "mcp_create_lists"))]
    pub(crate) async fn create_lists(
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

        // Literal parent_list_id values must belong to the caller. Parent refs
        // resolved within this batch point at rows created by this same user.
        let parent_list_ids: Vec<&str> = p
            .lists
            .iter()
            .filter_map(|l| l.parent_list_id.as_deref())
            .collect();
        self.ensure_lists_owned(&uid, &parent_list_ids, "parent_list_id", &locale)
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

        // Parse every entry's list type up front, with the same rules and
        // defaults as `lists::create` (`checklist` when omitted, default
        // features when the entry lists none). An invalid type rejects the
        // whole batch before anything is written.
        let mut resolved = Vec::with_capacity(p.lists.len());
        for l in &p.lists {
            resolved.push(
                domain::lists::resolve_list_type(l.list_type.as_deref(), l.features.as_deref())
                    .map_err(self.domain_err(&locale))?,
            );
        }

        let mut tx = self.pool.begin().await.map_err(self.sqlx_err(&locale))?;
        let mut resolver = RefResolver::new();
        let mut result = Vec::with_capacity(p.lists.len());
        // (container_id, parent_list_id, normalized_name) → id
        let mut seen: std::collections::HashMap<(Option<String>, Option<String>, String), String> =
            std::collections::HashMap::new();

        for (list, (list_type, feature_names)) in p.lists.iter().zip(resolved) {
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

            let norm_key = (
                container_id.map(str::to_owned),
                parent_list_id.map(str::to_owned),
                list.name.trim().to_lowercase(),
            );
            if let Some(eid) = seen.get(&norm_key) {
                resolver
                    .register(list.client_ref.as_deref(), eid)
                    .map_err(|e| self.map_err(McpError::BadRequest(e.to_string()), &locale))?;
                result.push(serde_json::json!({"id": eid, "name": list.name, "existed": true}));
                continue;
            }
            if let Some(eid) = db::lists::find_id_by_name_in_scope(
                &mut *tx,
                &uid,
                &list.name,
                container_id,
                parent_list_id,
                None,
            )
            .await
            .map_err(self.db_err(&locale))?
            {
                seen.insert(norm_key, eid.clone());
                resolver
                    .register(list.client_ref.as_deref(), &eid)
                    .map_err(|e| self.map_err(McpError::BadRequest(e.to_string()), &locale))?;
                result.push(serde_json::json!({"id": eid, "name": list.name, "existed": true}));
                continue;
            }

            let new_id = Uuid::new_v4().to_string();

            domain::lists::insert_list_in(
                &mut tx,
                &InsertListInput {
                    id: new_id.clone(),
                    user_id: uid.clone(),
                    position,
                    name: list.name.clone(),
                    icon: list.icon.clone(),
                    color: list.color.clone(),
                    description: list.description.clone(),
                    list_type: list_type.as_str().to_owned(),
                    container_id: container_id.map(str::to_owned),
                    parent_list_id: parent_list_id.map(str::to_owned),
                },
                &feature_names,
            )
            .await
            .map_err(self.domain_err(&locale))?;

            seen.insert(norm_key, new_id.clone());
            resolver
                .register(list.client_ref.as_deref(), &new_id)
                .map_err(|e| self.map_err(McpError::BadRequest(e.to_string()), &locale))?;

            result.push(serde_json::json!({"id": new_id, "name": list.name}));
        }

        tx.commit().await.map_err(self.sqlx_err(&locale))?;
        self.json_result(result, &locale)
    }

    // ── Read-only resource-compat tools ──────────────────────────────────────

    #[rmcp::tool(name = "list_lists", description = "mcp-tool-list_lists-desc")]
    pub(crate) async fn list_lists(
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
    pub(crate) async fn get_list(
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

    // ── Movement tools ────────────────────────────────────────────────────────

    #[rmcp::tool(
        name = "move_list_to_container",
        description = "mcp-tool-move_list_to_container-desc"
    )]
    #[tracing::instrument(skip(self, parts), fields(action = "mcp_move_list_to_container"))]
    pub(crate) async fn move_list_to_container(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<MoveListToContainerParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        if let Some(ref cid) = p.container_id {
            domain::containers::get_one(&self.pool, cid, &uid)
                .await
                .map_err(self.domain_err(&locale))?;
        }
        let position = db::lists::next_position(&self.pool, &uid, p.container_id.as_deref(), None)
            .await
            .map_err(self.db_err(&locale))?;
        let req = domain::lists::MoveListRequest {
            position,
            container_id: p.container_id,
            parent_list_id: None,
        };
        let list = domain::lists::move_list(&self.pool, &p.list_id, &uid, &req)
            .await
            .map_err(self.domain_err(&locale))?
            .ok_or_else(|| {
                self.map_err(
                    McpError::Domain(domain::DomainError::NotFound("list")),
                    &locale,
                )
            })?;
        self.json_result(list, &locale)
    }
}
