use http::request::Parts;
use kartoteka_db as db;
use kartoteka_domain as domain;
use kartoteka_shared::types::{
    CreateContainerRequest, MoveContainerRequest, UpdateContainerRequest,
};
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
use crate::server::{
    KartotekaServer, batch::PositionAllocator, decode_cursor, encode_cursor, localize_item,
};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateContainerParams {
    pub name: String,
    pub icon: Option<String>,
    /// Accent colour as `#rrggbb` (null = neutral).
    pub color: Option<String>,
    pub description: Option<String>,
    /// null/omit = folder; "active" | "done" | "paused" = project
    pub status: Option<String>,
    pub parent_container_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateContainersInput {
    pub name: String,
    pub icon: Option<String>,
    /// Accent colour as `#rrggbb` (null = neutral).
    pub color: Option<String>,
    pub description: Option<String>,
    /// null/omit = folder; "active" | "done" | "paused" = project
    pub status: Option<String>,
    /// Real UUID of an existing parent container.
    pub parent_container_id: Option<String>,
    /// `client_ref` of a container created earlier in this same batch.
    pub parent_container_ref: Option<String>,
    /// Label that other entries in this batch can reference via `parent_container_ref`.
    pub client_ref: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateContainersParams {
    pub containers: Vec<CreateContainersInput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetContainerStatusParams {
    pub container_id: String,
    /// null = folder; "active" | "done" | "paused" = project.
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetContainerParams {
    pub container_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListContainerItemsParams {
    pub container_id: String,
    /// Include items from nested sub-containers (default false)
    pub recursive: Option<bool>,
    /// Opaque cursor from previous response; omit for first page
    pub cursor: Option<String>,
    /// Max items to return (1–500, default 100)
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveContainerToParentParams {
    pub container_id: String,
    /// ID of the parent container to nest this container under. Pass null to move the container to root level.
    pub parent_container_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateContainerParams {
    pub container_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    /// Icon: `lucide:<name>`, `emoji:<e>` or a bare emoji.
    pub icon: Option<String>,
    /// null = folder; "active" | "done" | "paused" = project.
    pub status: Option<String>,
    /// Field names to explicitly clear (set to null). Valid values: description,
    /// icon, status
    pub clear: Option<Vec<String>>,
}

/// `None` keeps the stored value, `Some(None)` clears it, `Some(Some(v))` sets it.
fn nullable<T>(val: Option<T>, cleared: bool) -> Option<Option<T>> {
    if cleared { Some(None) } else { val.map(Some) }
}

impl UpdateContainerParams {
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
    fn to_request(&self) -> Result<UpdateContainerRequest, McpError> {
        self.ensure_not_both("description", self.description.is_some())?;
        self.ensure_not_both("icon", self.icon.is_some())?;
        self.ensure_not_both("status", self.status.is_some())?;
        Ok(UpdateContainerRequest {
            name: self.name.clone(),
            icon: nullable(self.icon.clone(), self.cleared("icon")),
            color: None,
            description: nullable(self.description.clone(), self.cleared("description")),
            status: nullable(self.status.clone(), self.cleared("status")),
            location_id: None,
        })
    }
}

#[tool_router(router = containers_router, vis = "pub(crate)")]
impl KartotekaServer {
    // ── New tools: create_container + batch creates ───────────────────────────

    #[rmcp::tool(
        name = "create_container",
        description = "mcp-tool-create_container-desc"
    )]
    #[tracing::instrument(skip(self, parts), fields(action = "mcp_create_container"))]
    pub(crate) async fn create_container(
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
            color: p.color,
        };
        let container = domain::containers::create(&self.pool, &uid, &req)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(container, &locale)
    }

    #[rmcp::tool(
        name = "create_containers",
        description = "mcp-tool-create_containers-desc"
    )]
    #[tracing::instrument(skip(self, parts), fields(action = "mcp_create_containers"))]
    pub(crate) async fn create_containers(
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
                color: container.color.clone(),
            };

            // Same rules as the single `create_container` tool
            // (domain::containers::create): status plus "parent must be an
            // owned folder". Validated in-tx because a parent created earlier
            // in this batch only exists inside the transaction.
            domain::containers::validate_create_container(&mut *tx, &uid, &req)
                .await
                .map_err(self.domain_err(&locale))?;

            let inserted =
                domain::containers::insert_container_in(&mut tx, &new_id, &uid, &req, position)
                    .await
                    .map_err(self.domain_err(&locale))?;

            resolver
                .register(container.client_ref.as_deref(), &new_id)
                .map_err(|e| self.map_err(McpError::BadRequest(e.to_string()), &locale))?;

            result.push(serde_json::json!({"id": inserted.id, "name": inserted.name}));
        }

        tx.commit().await.map_err(self.sqlx_err(&locale))?;
        self.json_result(result, &locale)
    }

    #[rmcp::tool(
        name = "list_containers",
        description = "mcp-tool-list_containers-desc"
    )]
    pub(crate) async fn list_containers(
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
    pub(crate) async fn get_container(
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

    #[rmcp::tool(
        name = "set_container_status",
        description = "mcp-tool-set_container_status-desc"
    )]
    #[tracing::instrument(skip(self, parts), fields(action = "mcp_set_container_status"))]
    pub(crate) async fn set_container_status(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<SetContainerStatusParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let container =
            domain::containers::set_status(&self.pool, &p.container_id, &uid, p.status.as_deref())
                .await
                .map_err(self.domain_err(&locale))?;
        self.json_result(container, &locale)
    }

    #[rmcp::tool(
        name = "list_container_items",
        description = "mcp-tool-list_container_items-desc"
    )]
    pub(crate) async fn list_container_items(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<ListContainerItemsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let tz = Self::timezone(&parts);
        tracing::debug!(container_id = %p.container_id, recursive = p.recursive.unwrap_or(false));
        let limit = domain::paging::clamp_limit(p.limit);
        let offset = decode_cursor(p.cursor.as_deref());
        let recursive = p.recursive.unwrap_or(false);
        let mut rows = domain::items::list_for_container(
            &self.pool,
            &p.container_id,
            &uid,
            limit + 1,
            offset,
            recursive,
        )
        .await
        .map_err(self.domain_err(&locale))?;
        let next_cursor = if rows.len() > limit as usize {
            rows.truncate(limit as usize);
            Some(encode_cursor(offset + limit as usize))
        } else {
            None
        };
        let rows: Vec<_> = rows
            .into_iter()
            .map(|mut i| {
                i.item = localize_item(i.item, &tz);
                i
            })
            .collect();
        self.json_result(
            domain::paging::Paged {
                data: rows,
                next_cursor,
                limit,
            },
            &locale,
        )
    }

    #[rmcp::tool(
        name = "move_container_to_parent",
        description = "mcp-tool-move_container_to_parent-desc"
    )]
    #[tracing::instrument(skip(self, parts), fields(action = "mcp_move_container_to_parent"))]
    pub(crate) async fn move_container_to_parent(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<MoveContainerToParentParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let req = MoveContainerRequest {
            parent_container_id: p.parent_container_id,
            position: None,
        };
        let container = domain::containers::move_container(&self.pool, &p.container_id, &uid, &req)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(container, &locale)
    }

    #[rmcp::tool(
        name = "update_container",
        description = "mcp-tool-update_container-desc"
    )]
    #[tracing::instrument(
        skip(self, parts, p),
        fields(action = "mcp_update_container", container_id = %p.container_id)
    )]
    pub(crate) async fn update_container(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<UpdateContainerParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let req = p.to_request().map_err(|e| self.map_err(e, &locale))?;
        let container = domain::containers::update(&self.pool, &p.container_id, &uid, &req)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(container, &locale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::test_support::*;
    use kartoteka_db::test_helpers::create_test_user;
    use serde_json::json;

    fn update_params(container_id: &str) -> UpdateContainerParams {
        UpdateContainerParams {
            container_id: container_id.to_owned(),
            name: None,
            description: None,
            icon: None,
            status: None,
            clear: None,
        }
    }

    #[tokio::test]
    async fn update_container_updates_and_clears_fields() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let container =
            domain::containers::create(&server.pool, &uid, &domain_container_req("Stary"))
                .await
                .unwrap();

        let updated = server
            .update_container(
                Extension(parts_for_user(&uid)),
                Parameters(UpdateContainerParams {
                    name: Some("Nowy".to_owned()),
                    description: Some("Opis".to_owned()),
                    icon: Some("lucide:folder".to_owned()),
                    status: Some("active".to_owned()),
                    ..update_params(&container.id)
                }),
            )
            .await
            .unwrap();
        let body = item_json(&updated).await;
        assert_eq!(body["name"], "Nowy");
        assert_eq!(body["description"], "Opis");
        assert_eq!(body["icon"], "lucide:folder");
        assert_eq!(body["status"], "active");

        // clear turns the project back into a folder and drops the description
        let cleared = server
            .update_container(
                Extension(parts_for_user(&uid)),
                Parameters(UpdateContainerParams {
                    clear: Some(vec!["description".to_owned(), "status".to_owned()]),
                    ..update_params(&container.id)
                }),
            )
            .await
            .unwrap();
        let body = item_json(&cleared).await;
        assert_eq!(body["name"], "Nowy");
        assert!(body["description"].is_null());
        assert!(body["status"].is_null());
    }

    #[test]
    fn update_container_rejects_a_field_that_is_set_and_cleared() {
        for name in ["description", "icon", "status"] {
            let mut params = update_params("container-1");
            match name {
                "description" => params.description = Some("both".to_owned()),
                "icon" => params.icon = Some("lucide:folder".to_owned()),
                _ => params.status = Some("active".to_owned()),
            }
            params.clear = Some(vec![name.to_owned()]);
            assert!(
                matches!(params.to_request(), Err(McpError::BadRequest(_))),
                "{name} set and cleared must be rejected"
            );
        }
    }

    #[tokio::test]
    async fn update_container_to_an_unknown_status_is_rejected() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let server = test_server(pool);
        let container = domain::containers::create(&server.pool, &uid, &domain_container_req("C"))
            .await
            .unwrap();

        let err = server
            .update_container(
                Extension(parts_for_user(&uid)),
                Parameters(UpdateContainerParams {
                    status: Some("blocked".to_owned()),
                    ..update_params(&container.id)
                }),
            )
            .await
            .unwrap_err();
        assert_eq!(
            err.data,
            Some(json!({ "error": "validation", "reason": "invalid_container_status" }))
        );
    }

    #[tokio::test]
    async fn update_container_of_another_user_is_not_found() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let intruder = create_test_user(&pool).await;
        let server = test_server(pool);
        let container =
            domain::containers::create(&server.pool, &uid, &domain_container_req("Moje"))
                .await
                .unwrap();

        let err = server
            .update_container(
                Extension(parts_for_user(&intruder)),
                Parameters(UpdateContainerParams {
                    name: Some("Przejęte".to_owned()),
                    ..update_params(&container.id)
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(
            err.data,
            Some(json!({ "error": "not_found", "entity": "container" }))
        );
        let row = domain::containers::get_one(&server.pool, &container.id, &uid)
            .await
            .unwrap();
        assert_eq!(row.name, "Moje");
    }
}
