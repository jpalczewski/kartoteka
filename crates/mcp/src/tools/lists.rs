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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReorderListsParams {
    /// Container whose lists are being reordered. Omit for root-level lists.
    pub container_id: Option<String>,
    /// Parent list whose sublists are being reordered. Omit for root-level lists.
    pub parent_list_id: Option<String>,
    /// Every list id of that scope, in the new order.
    pub list_ids: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateListParams {
    pub list_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    /// Icon: `lucide:<name>`, `emoji:<e>` or a bare emoji.
    pub icon: Option<String>,
    /// checklist (default), shopping, schedule, log, notes. Changing the type does
    /// not change the features — use set_list_features for those.
    pub list_type: Option<String>,
    /// Field names to explicitly clear (set to null). Valid values: description, icon
    pub clear: Option<Vec<String>>,
}

/// `None` keeps the stored value, `Some(None)` clears it, `Some(Some(v))` sets it.
fn nullable<T>(val: Option<T>, cleared: bool) -> Option<Option<T>> {
    if cleared { Some(None) } else { val.map(Some) }
}

impl UpdateListParams {
    fn cleared(&self, name: &str) -> bool {
        self.clear
            .as_deref()
            .is_some_and(|v| v.iter().any(|s| s == name))
    }

    /// A field cannot be given a value and cleared in the same call.
    fn ensure_not_both(&self, name: &str, set: bool) -> Result<(), McpError> {
        if set && self.cleared(name) {
            return Err(McpError::BadRequest(format!(
                "{name} is both set and listed in clear"
            )));
        }
        Ok(())
    }

    /// Build the domain request from the params.
    fn to_request(&self) -> Result<domain::lists::UpdateListRequest, McpError> {
        self.ensure_not_both("description", self.description.is_some())?;
        self.ensure_not_both("icon", self.icon.is_some())?;
        Ok(domain::lists::UpdateListRequest {
            name: self.name.clone(),
            icon: nullable(self.icon.clone(), self.cleared("icon")),
            color: None,
            description: nullable(self.description.clone(), self.cleared("description")),
            list_type: self.list_type.clone(),
            location_id: None,
        })
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetListFeaturesParams {
    pub list_id: String,
    /// The complete set of feature names to enable. Enabling a feature keeps its
    /// config if it was already on; disabled features lose theirs.
    pub features: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateListFeatureConfigParams {
    pub list_id: String,
    /// Feature name, e.g. "deadlines". Must already be enabled on the list.
    pub feature: String,
    /// Feature configuration object (replaces the stored one).
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetArchivedParams {
    /// "list" or "container". Archiving a container also archives its
    /// sub-containers and their lists.
    pub entity_type: String,
    pub entity_id: String,
    /// true archives, false restores. Repeating the call is safe.
    pub archived: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetPinnedParams {
    /// "list" or "container".
    pub entity_type: String,
    pub entity_id: String,
    /// true pins, false unpins. Repeating the call is safe.
    pub pinned: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResetListParams {
    pub list_id: String,
}

/// The entity kinds `set_archived` / `set_pinned` accept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityKind {
    List,
    Container,
}

impl EntityKind {
    fn parse(raw: &str) -> Result<Self, domain::DomainError> {
        match raw {
            "list" => Ok(Self::List),
            "container" => Ok(Self::Container),
            _ => Err(domain::DomainError::Validation("invalid_entity_type")),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Container => "container",
        }
    }
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

    // ── Lifecycle tools ──────────────────────────────────────────────────────

    #[rmcp::tool(name = "update_list", description = "mcp-tool-update_list-desc")]
    #[tracing::instrument(
        skip(self, parts, p),
        fields(action = "mcp_update_list", list_id = %p.list_id)
    )]
    pub(crate) async fn update_list(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<UpdateListParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let req = p.to_request().map_err(|e| self.map_err(e, &locale))?;
        let list = domain::lists::update(&self.pool, &p.list_id, &uid, &req)
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

    #[rmcp::tool(
        name = "set_list_features",
        description = "mcp-tool-set_list_features-desc"
    )]
    #[tracing::instrument(
        skip(self, parts, p),
        fields(action = "mcp_set_list_features", list_id = %p.list_id)
    )]
    pub(crate) async fn set_list_features(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<SetListFeaturesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let req = domain::lists::SetFeaturesRequest {
            features: p.features,
        };
        let list = domain::lists::set_features(&self.pool, &p.list_id, &uid, &req)
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

    #[rmcp::tool(
        name = "update_list_feature_config",
        description = "mcp-tool-update_list_feature_config-desc"
    )]
    #[tracing::instrument(
        skip(self, parts, p),
        fields(action = "mcp_update_list_feature_config", list_id = %p.list_id, feature = %p.feature)
    )]
    pub(crate) async fn update_list_feature_config(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<UpdateListFeatureConfigParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        domain::lists::update_feature_config(&self.pool, &p.list_id, &uid, &p.feature, p.config)
            .await
            .map_err(self.domain_err(&locale))?;
        let list = domain::lists::get_one(&self.pool, &p.list_id, &uid)
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

    #[rmcp::tool(name = "set_archived", description = "mcp-tool-set_archived-desc")]
    #[tracing::instrument(
        skip(self, parts, p),
        fields(action = "mcp_set_archived", entity_type = %p.entity_type, entity_id = %p.entity_id)
    )]
    pub(crate) async fn set_archived(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<SetArchivedParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let kind =
            EntityKind::parse(&p.entity_type).map_err(|e| self.map_err(e.into(), &locale))?;
        let archived = p.archived;
        match kind {
            EntityKind::List => {
                let list = domain::lists::set_archived(&self.pool, &p.entity_id, &uid, archived)
                    .await
                    .map_err(self.domain_err(&locale))?
                    .ok_or_else(|| self.not_found(kind, &locale))?;
                self.json_result(list, &locale)
            }
            EntityKind::Container => {
                let container =
                    domain::containers::set_archived(&self.pool, &p.entity_id, &uid, archived)
                        .await
                        .map_err(self.domain_err(&locale))?
                        .ok_or_else(|| self.not_found(kind, &locale))?;
                self.json_result(container, &locale)
            }
        }
    }

    #[rmcp::tool(name = "set_pinned", description = "mcp-tool-set_pinned-desc")]
    #[tracing::instrument(
        skip(self, parts, p),
        fields(action = "mcp_set_pinned", entity_type = %p.entity_type, entity_id = %p.entity_id)
    )]
    pub(crate) async fn set_pinned(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<SetPinnedParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let kind =
            EntityKind::parse(&p.entity_type).map_err(|e| self.map_err(e.into(), &locale))?;
        let pinned = p.pinned;
        match kind {
            EntityKind::List => {
                let list = domain::lists::set_pinned(&self.pool, &p.entity_id, &uid, pinned)
                    .await
                    .map_err(self.domain_err(&locale))?
                    .ok_or_else(|| self.not_found(kind, &locale))?;
                self.json_result(list, &locale)
            }
            EntityKind::Container => {
                let container =
                    domain::containers::set_pinned(&self.pool, &p.entity_id, &uid, pinned)
                        .await
                        .map_err(self.domain_err(&locale))?
                        .ok_or_else(|| self.not_found(kind, &locale))?;
                self.json_result(container, &locale)
            }
        }
    }

    #[rmcp::tool(name = "reset_list", description = "mcp-tool-reset_list-desc")]
    #[tracing::instrument(
        skip(self, parts, p),
        fields(action = "mcp_reset_list", list_id = %p.list_id)
    )]
    pub(crate) async fn reset_list(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<ResetListParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let reset = domain::lists::reset(&self.pool, &p.list_id, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(serde_json::json!({ "reset": reset }), &locale)
    }

    #[rmcp::tool(name = "list_archived", description = "mcp-tool-list_archived-desc")]
    pub(crate) async fn list_archived(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let lists = domain::lists::list_archived(&self.pool, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        let containers = domain::containers::list_archived(&self.pool, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(
            serde_json::json!({ "lists": lists, "containers": containers }),
            &locale,
        )
    }

    /// `NotFound` for the entity kind a lifecycle tool was asked about.
    fn not_found(&self, kind: EntityKind, locale: &str) -> ErrorData {
        self.map_err(
            McpError::Domain(domain::DomainError::NotFound(kind.label())),
            locale,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::test_support::*;
    use kartoteka_db::test_helpers::create_test_user;
    use kartoteka_shared::types::CreateContainerRequest;
    use serde_json::json;
    use sqlx::SqlitePool;

    async fn make_container(pool: &SqlitePool, uid: &str, name: &str) -> String {
        domain::containers::create(pool, uid, &domain_container_req(name))
            .await
            .unwrap()
            .id
    }

    async fn make_child_container(
        pool: &SqlitePool,
        uid: &str,
        name: &str,
        parent: &str,
    ) -> String {
        domain::containers::create(
            pool,
            uid,
            &CreateContainerRequest {
                parent_container_id: Some(parent.to_owned()),
                ..domain_container_req(name)
            },
        )
        .await
        .unwrap()
        .id
    }

    fn update_params(list_id: &str) -> UpdateListParams {
        UpdateListParams {
            list_id: list_id.to_owned(),
            name: None,
            description: None,
            icon: None,
            list_type: None,
            clear: None,
        }
    }

    #[tokio::test]
    async fn update_list_renames_and_clears_the_description() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let list_id = insert_test_list(&server.pool, &uid).await;

        let renamed = server
            .update_list(
                Extension(parts_for_user(&uid)),
                Parameters(UpdateListParams {
                    name: Some("Renamed".to_owned()),
                    description: Some("A real description".to_owned()),
                    ..update_params(&list_id)
                }),
            )
            .await
            .unwrap();
        let body = item_json(&renamed).await;
        assert_eq!(body["name"], "Renamed");
        assert_eq!(body["description"], "A real description");

        let cleared = server
            .update_list(
                Extension(parts_for_user(&uid)),
                Parameters(UpdateListParams {
                    clear: Some(vec!["description".to_owned()]),
                    ..update_params(&list_id)
                }),
            )
            .await
            .unwrap();
        let body = item_json(&cleared).await;
        assert_eq!(body["name"], "Renamed");
        assert!(body["description"].is_null(), "description was cleared");
    }

    /// #342: changing the type must not rewrite the feature set.
    #[tokio::test]
    async fn update_list_type_keeps_the_features() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let list_id = insert_test_list_with_features(&server.pool, &uid, &["checklist"]).await;

        let updated = server
            .update_list(
                Extension(parts_for_user(&uid)),
                Parameters(UpdateListParams {
                    list_type: Some("notes".to_owned()),
                    ..update_params(&list_id)
                }),
            )
            .await
            .unwrap();
        let body = item_json(&updated).await;
        assert_eq!(body["list_type"], "notes");
        let names: Vec<&str> = body["features"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|f| f["feature_name"].as_str())
            .collect();
        assert_eq!(names, vec!["checklist"]);
    }

    #[test]
    fn update_list_rejects_a_field_that_is_set_and_cleared() {
        let params = UpdateListParams {
            description: Some("both".to_owned()),
            clear: Some(vec!["description".to_owned()]),
            ..update_params("list-1")
        };
        assert!(matches!(params.to_request(), Err(McpError::BadRequest(_))));

        let params = UpdateListParams {
            icon: Some("lucide:star".to_owned()),
            clear: Some(vec!["icon".to_owned()]),
            ..update_params("list-1")
        };
        assert!(matches!(params.to_request(), Err(McpError::BadRequest(_))));
    }

    #[tokio::test]
    async fn update_list_to_a_duplicate_name_reports_already_exists() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let container_id = make_container(&server.pool, &uid, "Maj").await;
        let existing = domain::lists::create(
            &server.pool,
            &uid,
            &domain_list_req("Wakacje 2026", Some(container_id.clone())),
        )
        .await
        .unwrap();
        let other = domain::lists::create(
            &server.pool,
            &uid,
            &domain_list_req("Zima 2026", Some(container_id)),
        )
        .await
        .unwrap();

        let err = server
            .update_list(
                Extension(parts_for_user(&uid)),
                Parameters(UpdateListParams {
                    name: Some("Wakacje 2026".to_owned()),
                    ..update_params(&other.id)
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(err.code.0, -32602, "unexpected code: {err:?}");
        let data = err.data.as_ref().expect("already_exists carries data");
        assert_eq!(data["error"], "already_exists");
        assert_eq!(data["id"], existing.id.as_str());
    }

    #[tokio::test]
    async fn set_list_features_keeps_the_config_of_a_feature_that_stays_enabled() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let list_id =
            insert_test_list_with_features(&server.pool, &uid, &["checklist", "deadlines"]).await;

        let configured = server
            .update_list_feature_config(
                Extension(parts_for_user(&uid)),
                Parameters(UpdateListFeatureConfigParams {
                    list_id: list_id.clone(),
                    feature: "deadlines".to_owned(),
                    config: json!({ "warn_days": 3 }),
                }),
            )
            .await
            .unwrap();
        assert_eq!(item_json(&configured).await["id"], list_id.as_str());

        let updated = server
            .set_list_features(
                Extension(parts_for_user(&uid)),
                Parameters(SetListFeaturesParams {
                    list_id: list_id.clone(),
                    features: vec![
                        "checklist".to_owned(),
                        "deadlines".to_owned(),
                        "location".to_owned(),
                    ],
                }),
            )
            .await
            .unwrap();
        let body = item_json(&updated).await;
        let features = body["features"].as_array().unwrap();
        let deadlines = features
            .iter()
            .find(|f| f["feature_name"] == "deadlines")
            .expect("deadlines stays enabled");
        assert_eq!(deadlines["config"], json!({ "warn_days": 3 }));
        let location = features
            .iter()
            .find(|f| f["feature_name"] == "location")
            .expect("location is enabled");
        assert_eq!(location["config"], json!({}));
    }

    #[tokio::test]
    async fn set_list_features_rejects_an_unknown_name() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let list_id = insert_test_list(&server.pool, &uid).await;

        let err = server
            .set_list_features(
                Extension(parts_for_user(&uid)),
                Parameters(SetListFeaturesParams {
                    list_id,
                    features: vec!["teleport".to_owned()],
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(
            err.data,
            Some(json!({ "error": "validation", "reason": "unknown_feature" }))
        );
    }

    #[tokio::test]
    async fn set_archived_is_idempotent_for_lists() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let list_id = insert_test_list(&server.pool, &uid).await;

        for _ in 0..2 {
            let archived = server
                .set_archived(
                    Extension(parts_for_user(&uid)),
                    Parameters(SetArchivedParams {
                        entity_type: "list".to_owned(),
                        entity_id: list_id.clone(),
                        archived: true,
                    }),
                )
                .await
                .unwrap();
            assert_eq!(item_json(&archived).await["archived"], true);
        }

        let lists = server
            .list_lists(Extension(parts_for_user(&uid)))
            .await
            .unwrap();
        assert_eq!(
            item_json(&lists).await,
            json!([]),
            "archived lists are hidden"
        );

        let restored = server
            .set_archived(
                Extension(parts_for_user(&uid)),
                Parameters(SetArchivedParams {
                    entity_type: "list".to_owned(),
                    entity_id: list_id.clone(),
                    archived: false,
                }),
            )
            .await
            .unwrap();
        assert_eq!(item_json(&restored).await["archived"], false);
    }

    #[tokio::test]
    async fn set_pinned_is_idempotent_for_containers() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let container_id = make_container(&server.pool, &uid, "Projekt").await;

        for _ in 0..2 {
            let pinned = server
                .set_pinned(
                    Extension(parts_for_user(&uid)),
                    Parameters(SetPinnedParams {
                        entity_type: "container".to_owned(),
                        entity_id: container_id.clone(),
                        pinned: true,
                    }),
                )
                .await
                .unwrap();
            assert_eq!(item_json(&pinned).await["pinned"], true);
        }

        let unpinned = server
            .set_pinned(
                Extension(parts_for_user(&uid)),
                Parameters(SetPinnedParams {
                    entity_type: "container".to_owned(),
                    entity_id: container_id,
                    pinned: false,
                }),
            )
            .await
            .unwrap();
        assert_eq!(item_json(&unpinned).await["pinned"], false);
    }

    #[tokio::test]
    async fn set_archived_and_set_pinned_reject_an_unknown_entity_type() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let list_id = insert_test_list(&server.pool, &uid).await;

        let err = server
            .set_archived(
                Extension(parts_for_user(&uid)),
                Parameters(SetArchivedParams {
                    entity_type: "item".to_owned(),
                    entity_id: list_id.clone(),
                    archived: true,
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(
            err.data,
            Some(json!({ "error": "validation", "reason": "invalid_entity_type" }))
        );

        let err = server
            .set_pinned(
                Extension(parts_for_user(&uid)),
                Parameters(SetPinnedParams {
                    entity_type: "tag".to_owned(),
                    entity_id: list_id,
                    pinned: true,
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(
            err.data,
            Some(json!({ "error": "validation", "reason": "invalid_entity_type" }))
        );
    }

    /// Every lifecycle tool is scoped to the caller: a foreign id is `not_found`
    /// for its entity kind and changes nothing.
    #[tokio::test]
    async fn lifecycle_tools_are_scoped_to_the_caller() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let intruder = create_test_user(&pool).await;
        let server = test_server(pool);
        let list_id = insert_test_list_with_features(&server.pool, &uid, &["checklist"]).await;
        let container_id = make_container(&server.pool, &uid, "Cudzy").await;

        let list_not_found = Some(json!({ "error": "not_found", "entity": "list" }));
        let container_not_found = Some(json!({ "error": "not_found", "entity": "container" }));

        let err = server
            .update_list(
                Extension(parts_for_user(&intruder)),
                Parameters(UpdateListParams {
                    name: Some("Hijacked".to_owned()),
                    ..update_params(&list_id)
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(err.data, list_not_found);

        let err = server
            .set_list_features(
                Extension(parts_for_user(&intruder)),
                Parameters(SetListFeaturesParams {
                    list_id: list_id.clone(),
                    features: vec!["checklist".to_owned()],
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(err.data, list_not_found);

        let err = server
            .update_list_feature_config(
                Extension(parts_for_user(&intruder)),
                Parameters(UpdateListFeatureConfigParams {
                    list_id: list_id.clone(),
                    feature: "checklist".to_owned(),
                    config: json!({}),
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(err.data, list_not_found);

        let err = server
            .set_archived(
                Extension(parts_for_user(&intruder)),
                Parameters(SetArchivedParams {
                    entity_type: "list".to_owned(),
                    entity_id: list_id.clone(),
                    archived: true,
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(err.data, list_not_found);

        let err = server
            .set_archived(
                Extension(parts_for_user(&intruder)),
                Parameters(SetArchivedParams {
                    entity_type: "container".to_owned(),
                    entity_id: container_id.clone(),
                    archived: true,
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(err.data, container_not_found);

        let err = server
            .set_pinned(
                Extension(parts_for_user(&intruder)),
                Parameters(SetPinnedParams {
                    entity_type: "list".to_owned(),
                    entity_id: list_id.clone(),
                    pinned: true,
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(err.data, list_not_found);

        let err = server
            .set_pinned(
                Extension(parts_for_user(&intruder)),
                Parameters(SetPinnedParams {
                    entity_type: "container".to_owned(),
                    entity_id: container_id,
                    pinned: true,
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(err.data, container_not_found);

        let err = server
            .reset_list(
                Extension(parts_for_user(&intruder)),
                Parameters(ResetListParams {
                    list_id: list_id.clone(),
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(err.data, list_not_found);

        // Nothing changed for the owner.
        let row = domain::lists::get_one(&server.pool, &list_id, &uid)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.name, "Test List");
        assert!(!row.archived);
        assert!(!row.pinned);
    }

    #[tokio::test]
    async fn reset_list_unchecks_items() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let list_id = insert_test_list(&server.pool, &uid).await;
        sqlx::query(
            "INSERT INTO items (id, list_id, title, completed, status) \
             VALUES ('i1', ?, 'A', 1, 'done'), ('i2', ?, 'B', 0, 'todo')",
        )
        .bind(&list_id)
        .bind(&list_id)
        .execute(&server.pool)
        .await
        .unwrap();

        let result = server
            .reset_list(
                Extension(parts_for_user(&uid)),
                Parameters(ResetListParams {
                    list_id: list_id.clone(),
                }),
            )
            .await
            .unwrap();
        assert_eq!(item_json(&result).await, json!({ "reset": 2 }));

        let rows: Vec<(String, Option<String>)> =
            sqlx::query_as("SELECT title, status FROM items WHERE list_id = ? AND completed = 0")
                .bind(&list_id)
                .fetch_all(&server.pool)
                .await
                .unwrap();
        assert_eq!(rows.len(), 2);
    }

    /// Archiving a container archives its sub-containers and their lists;
    /// `list_archived` reports the root of the archived subtree, while
    /// `list_lists` / `list_containers` hide the hidden rows.
    #[tokio::test]
    async fn archiving_a_container_cascades_and_list_archived_reports_the_root() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let parent = make_container(&server.pool, &uid, "Rodzina").await;
        let child = make_child_container(&server.pool, &uid, "Dzieci", &parent).await;
        let list = domain::lists::create(
            &server.pool,
            &uid,
            &domain_list_req("Zakupy", Some(child.clone())),
        )
        .await
        .unwrap();

        let archived = server
            .set_archived(
                Extension(parts_for_user(&uid)),
                Parameters(SetArchivedParams {
                    entity_type: "container".to_owned(),
                    entity_id: parent.clone(),
                    archived: true,
                }),
            )
            .await
            .unwrap();
        assert_eq!(item_json(&archived).await["archived"], true);

        // The cascade reached the sub-container and its list.
        let child_row = domain::containers::get_one(&server.pool, &child, &uid)
            .await
            .unwrap();
        assert!(child_row.archived);
        let list_row = domain::lists::get_one(&server.pool, &list.id, &uid)
            .await
            .unwrap()
            .unwrap();
        assert!(list_row.archived);

        let archived_json = item_json(
            &server
                .list_archived(Extension(parts_for_user(&uid)))
                .await
                .unwrap(),
        )
        .await;
        let container_ids: Vec<&str> = archived_json["containers"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|c| c["id"].as_str())
            .collect();
        assert_eq!(
            container_ids,
            vec![parent.as_str()],
            "only the subtree root"
        );
        assert_eq!(
            archived_json["lists"],
            json!([]),
            "the list came via cascade"
        );

        let listed = item_json(
            &server
                .list_lists(Extension(parts_for_user(&uid)))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(listed, json!([]), "list_lists hides cascade-archived lists");
        let containers = item_json(
            &server
                .list_containers(Extension(parts_for_user(&uid)))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(containers, json!([]), "list_containers hides archived rows");

        // A foreign user sees nothing of it.
        let intruder = create_test_user(&server.pool).await;
        let foreign = item_json(
            &server
                .list_archived(Extension(parts_for_user(&intruder)))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(foreign, json!({ "lists": [], "containers": [] }));

        // Restoring brings the whole subtree back.
        server
            .set_archived(
                Extension(parts_for_user(&uid)),
                Parameters(SetArchivedParams {
                    entity_type: "container".to_owned(),
                    entity_id: parent,
                    archived: false,
                }),
            )
            .await
            .unwrap();
        let restored = item_json(
            &server
                .list_archived(Extension(parts_for_user(&uid)))
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(restored, json!({ "lists": [], "containers": [] }));
        assert_eq!(
            item_json(
                &server
                    .list_lists(Extension(parts_for_user(&uid)))
                    .await
                    .unwrap(),
            )
            .await
            .as_array()
            .unwrap()
            .len(),
            1
        );
    }
}

// A second router block, because rmcp derives one router method per block: the
// ordering tools keep their own so this file stays a leaf for parallel edits.
#[tool_router(router = lists_order_router, vis = "pub(crate)")]
impl KartotekaServer {
    #[rmcp::tool(name = "reorder_lists", description = "mcp-tool-reorder_lists-desc")]
    #[tracing::instrument(
        skip(self, parts, p),
        fields(action = "mcp_reorder_lists", count = p.list_ids.len())
    )]
    pub(crate) async fn reorder_lists(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<ReorderListsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        // A list sits either in a container or under a parent list, never in both,
        // so two scope selectors describe no real scope at all.
        if p.container_id.is_some() && p.parent_list_id.is_some() {
            return Err(self.map_err(
                McpError::BadRequest(
                    "container_id and parent_list_id are mutually exclusive".to_owned(),
                ),
                &locale,
            ));
        }
        // Scope ownership first, so a foreign container or parent reads as
        // `not_found` instead of an empty scope.
        domain::lists::ensure_placement_owned(
            &self.pool,
            &uid,
            p.container_id.as_deref(),
            p.parent_list_id.as_deref(),
        )
        .await
        .map_err(self.domain_err(&locale))?;
        let scope = db::lists::ids_in_scope(
            &self.pool,
            &uid,
            p.container_id.as_deref(),
            p.parent_list_id.as_deref(),
        )
        .await
        .map_err(self.db_err(&locale))?;
        domain::DomainError::ensure_ids_in_scope(&scope, &p.list_ids, "list")
            .map_err(self.domain_err(&locale))?;
        // `lists::reorder` writes positions 0..n for exactly the ids it receives,
        // so a partial order would collide with the positions of the lists it
        // left out. The full set is required here and not in the domain layer:
        // the UI legitimately reorders only what it has on screen.
        if p.list_ids.len() != scope.len() {
            return Err(self.map_err(
                McpError::Domain(domain::DomainError::Validation("incomplete_order")),
                &locale,
            ));
        }
        domain::lists::reorder(
            &self.pool,
            &uid,
            p.container_id.as_deref(),
            p.parent_list_id.as_deref(),
            &p.list_ids,
        )
        .await
        .map_err(self.domain_err(&locale))?;
        self.json_result(serde_json::json!({"reordered": p.list_ids.len()}), &locale)
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use crate::server::test_support::{
        domain_container_req, domain_list_req, item_json, multi_conn_pool, parts_for_user,
        test_server,
    };
    use kartoteka_db::test_helpers::create_test_user;
    use rmcp::handler::server::tool::Extension;
    use rmcp::handler::server::wrapper::Parameters;
    use serde_json::json;

    async fn container(server: &KartotekaServer, uid: &str, name: &str) -> String {
        domain::containers::create(&server.pool, uid, &domain_container_req(name))
            .await
            .unwrap()
            .id
    }

    /// Create a list where `lists::create` puts it at the end of its scope, so
    /// positions follow creation order.
    async fn list(
        server: &KartotekaServer,
        uid: &str,
        name: &str,
        container_id: Option<&str>,
        parent_list_id: Option<&str>,
    ) -> String {
        let mut req = domain_list_req(name, container_id.map(str::to_owned));
        req.parent_list_id = parent_list_id.map(str::to_owned);
        domain::lists::create(&server.pool, uid, &req)
            .await
            .unwrap()
            .id
    }

    async fn scope_ids(
        server: &KartotekaServer,
        uid: &str,
        container_id: Option<&str>,
        parent_list_id: Option<&str>,
    ) -> Vec<String> {
        db::lists::ids_in_scope(&server.pool, uid, container_id, parent_list_id)
            .await
            .unwrap()
    }

    fn error_reason(err: &ErrorData) -> String {
        let data = err.data.as_ref().expect("mapped errors carry data");
        let field = |key: &str| data[key].as_str().unwrap_or("").to_owned();
        format!("{}/{}", field("error"), field("reason"))
    }

    #[tokio::test]
    async fn reorder_lists_persists_the_root_order() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let a = list(&server, &uid, "A", None, None).await;
        let b = list(&server, &uid, "B", None, None).await;
        let c = list(&server, &uid, "C", None, None).await;

        let result = server
            .reorder_lists(
                Extension(parts_for_user(&uid)),
                Parameters(ReorderListsParams {
                    container_id: None,
                    parent_list_id: None,
                    list_ids: vec![c.clone(), a.clone(), b.clone()],
                }),
            )
            .await
            .unwrap();

        assert_eq!(item_json(&result).await, json!({"reordered": 3}));
        assert_eq!(scope_ids(&server, &uid, None, None).await, vec![c, a, b]);
    }

    #[tokio::test]
    async fn reorder_lists_persists_the_order_inside_a_container() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let container_id = container(&server, &uid, "Projects").await;
        let a = list(&server, &uid, "A", Some(&container_id), None).await;
        let b = list(&server, &uid, "B", Some(&container_id), None).await;
        // A root list must not be dragged into the container's scope.
        let root = list(&server, &uid, "Root", None, None).await;

        let result = server
            .reorder_lists(
                Extension(parts_for_user(&uid)),
                Parameters(ReorderListsParams {
                    container_id: Some(container_id.clone()),
                    parent_list_id: None,
                    list_ids: vec![b.clone(), a.clone()],
                }),
            )
            .await
            .unwrap();

        assert_eq!(item_json(&result).await, json!({"reordered": 2}));
        assert_eq!(
            scope_ids(&server, &uid, Some(&container_id), None).await,
            vec![b, a.clone()]
        );
        assert_eq!(scope_ids(&server, &uid, None, None).await, vec![root]);
    }

    #[tokio::test]
    async fn reorder_lists_persists_the_sublist_order() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let parent = list(&server, &uid, "Parent", None, None).await;
        let a = list(&server, &uid, "A", None, Some(&parent)).await;
        let b = list(&server, &uid, "B", None, Some(&parent)).await;

        let result = server
            .reorder_lists(
                Extension(parts_for_user(&uid)),
                Parameters(ReorderListsParams {
                    container_id: None,
                    parent_list_id: Some(parent.clone()),
                    list_ids: vec![b.clone(), a.clone()],
                }),
            )
            .await
            .unwrap();

        assert_eq!(item_json(&result).await, json!({"reordered": 2}));
        assert_eq!(
            scope_ids(&server, &uid, None, Some(&parent)).await,
            vec![b, a]
        );
    }

    #[tokio::test]
    async fn reorder_lists_rejects_both_scope_parameters() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let container_id = container(&server, &uid, "Projects").await;
        let parent = list(&server, &uid, "Parent", None, None).await;

        let err = server
            .reorder_lists(
                Extension(parts_for_user(&uid)),
                Parameters(ReorderListsParams {
                    container_id: Some(container_id),
                    parent_list_id: Some(parent),
                    list_ids: Vec::new(),
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(err.code.0, -32602, "unexpected code: {err:?}");
        assert_eq!(
            error_reason(&err),
            "validation/container_id and parent_list_id are mutually exclusive"
        );
    }

    #[tokio::test]
    async fn reorder_lists_rejects_a_partial_order() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let a = list(&server, &uid, "A", None, None).await;
        let b = list(&server, &uid, "B", None, None).await;
        let c = list(&server, &uid, "C", None, None).await;

        let err = server
            .reorder_lists(
                Extension(parts_for_user(&uid)),
                Parameters(ReorderListsParams {
                    container_id: None,
                    parent_list_id: None,
                    list_ids: vec![c.clone(), a.clone()],
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(err.code.0, -32602, "unexpected code: {err:?}");
        assert_eq!(error_reason(&err), "validation/incomplete_order");
        assert_eq!(
            scope_ids(&server, &uid, None, None).await,
            vec![a, b, c],
            "nothing may move"
        );
    }

    #[tokio::test]
    async fn reorder_lists_rejects_a_list_from_another_scope() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let container_id = container(&server, &uid, "Projects").await;
        let inside = list(&server, &uid, "Inside", Some(&container_id), None).await;
        let other = list(&server, &uid, "Also inside", Some(&container_id), None).await;
        let root = list(&server, &uid, "Root", None, None).await;

        let err = server
            .reorder_lists(
                Extension(parts_for_user(&uid)),
                Parameters(ReorderListsParams {
                    container_id: Some(container_id.clone()),
                    parent_list_id: None,
                    list_ids: vec![inside.clone(), other.clone(), root],
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(err.code.0, -32602, "unexpected code: {err:?}");
        assert_eq!(err.data.as_ref().unwrap()["entity"], "list");
        assert_eq!(
            scope_ids(&server, &uid, Some(&container_id), None).await,
            vec![inside, other]
        );
    }
}
