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
mod tests {
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
