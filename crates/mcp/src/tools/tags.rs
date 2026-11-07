use http::request::Parts;
use kartoteka_db as db;
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
use crate::server::KartotekaServer;

#[derive(Deserialize, JsonSchema)]
pub struct CreateTagParams {
    pub name: String,
    pub color: Option<String>,
    /// Icon: `lucide:<name>`, `emoji:<e>` or a bare emoji.
    pub icon: Option<String>,
    pub parent_tag_id: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct AssignTagParams {
    /// "item", "list", or "container"
    pub entity_type: String,
    pub entity_id: String,
    pub tag_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct UnassignTagParams {
    /// "item", "list", or "container"
    pub entity_type: String,
    pub entity_id: String,
    pub tag_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateTagsInput {
    pub name: String,
    pub color: Option<String>,
    /// Icon: `lucide:<name>`, `emoji:<e>` or a bare emoji.
    pub icon: Option<String>,
    /// Real UUID of an existing parent tag.
    pub parent_tag_id: Option<String>,
    /// client_ref of a tag created earlier in this same batch.
    pub parent_tag_ref: Option<String>,
    /// Label that other entries in this batch can reference via parent_tag_ref.
    pub client_ref: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateTagsParams {
    pub tags: Vec<CreateTagsInput>,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetTagEntitiesParams {
    pub tag_id: String,
    /// Filter by entity type: "item" or "list". Omit to get both.
    pub entity_type: Option<String>,
}

#[tool_router(router = tags_router, vis = "pub(crate)")]
impl KartotekaServer {
    #[rmcp::tool(name = "list_tags", description = "mcp-tool-list_tags-desc")]
    pub(crate) async fn list_tags(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let data = domain::tags::list_all(&self.pool, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(data, &locale)
    }

    #[rmcp::tool(name = "create_tag", description = "mcp-tool-create_tag-desc")]
    pub(crate) async fn create_tag(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<CreateTagParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let req = domain::tags::CreateTagRequest {
            name: p.name,
            color: p.color,
            parent_tag_id: p.parent_tag_id,
            tag_type: None,
            icon: p.icon,
            metadata: None,
        };
        self.domain_or_existing(domain::tags::create(&self.pool, &uid, &req).await, &locale)
    }

    #[rmcp::tool(name = "assign_tag", description = "mcp-tool-assign_tag-desc")]
    pub(crate) async fn assign_tag(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<AssignTagParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        match p.entity_type.as_str() {
            "item" => domain::tags::assign_to_item(&self.pool, &uid, &p.entity_id, &p.tag_id)
                .await
                .map_err(self.domain_err(&locale))?,
            "list" => domain::tags::assign_to_list(&self.pool, &uid, &p.entity_id, &p.tag_id)
                .await
                .map_err(self.domain_err(&locale))?,
            "container" => {
                domain::tags::assign_to_container(&self.pool, &uid, &p.entity_id, &p.tag_id)
                    .await
                    .map_err(self.domain_err(&locale))?
            }
            _ => {
                return Err(self.map_err(
                    McpError::BadRequest("invalid_entity_type".to_string()),
                    &locale,
                ));
            }
        }
        self.json_result(serde_json::json!({"ok": true}), &locale)
    }

    #[rmcp::tool(name = "unassign_tag", description = "mcp-tool-unassign_tag-desc")]
    pub(crate) async fn unassign_tag(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<UnassignTagParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        match p.entity_type.as_str() {
            "item" => {
                domain::tags::remove_from_item(&self.pool, &uid, &p.entity_id, &p.tag_id)
                    .await
                    .map_err(self.domain_err(&locale))?;
            }
            "list" => {
                domain::tags::remove_from_list(&self.pool, &uid, &p.entity_id, &p.tag_id)
                    .await
                    .map_err(self.domain_err(&locale))?;
            }
            "container" => {
                domain::tags::remove_from_container(&self.pool, &uid, &p.entity_id, &p.tag_id)
                    .await
                    .map_err(self.domain_err(&locale))?;
            }
            _ => {
                return Err(self.map_err(
                    McpError::BadRequest("invalid_entity_type".to_string()),
                    &locale,
                ));
            }
        }
        self.json_result(serde_json::json!({"ok": true}), &locale)
    }

    #[rmcp::tool(name = "create_tags", description = "mcp-tool-create_tags-desc")]
    pub(crate) async fn create_tags(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<CreateTagsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;

        if p.tags.is_empty() {
            return self.json_result(Vec::<serde_json::Value>::new(), &locale);
        }

        let mut tx = self.pool.begin().await.map_err(self.sqlx_err(&locale))?;
        let mut resolver = RefResolver::new();
        let mut result = Vec::with_capacity(p.tags.len());
        // (parent_tag_id, normalized_name) → id
        let mut seen: std::collections::HashMap<(Option<String>, String), String> =
            std::collections::HashMap::new();

        for tag in &p.tags {
            let parent_id: Option<String> = resolver
                .pick(
                    tag.parent_tag_id.as_deref(),
                    tag.parent_tag_ref.as_deref(),
                    false,
                )
                .map_err(|e| self.map_err(McpError::BadRequest(e.to_string()), &locale))?
                .map(str::to_owned);

            // Same rules as the single `create_tag` tool (domain::tags::create):
            // name/colour validation and parent ownership. A parent created
            // earlier in this batch is visible inside the transaction.
            let req = domain::tags::CreateTagRequest {
                name: tag.name.clone(),
                icon: tag.icon.clone(),
                color: tag.color.clone(),
                parent_tag_id: parent_id.clone(),
                tag_type: None,
                metadata: None,
            };
            domain::tags::validate_create_tag(&mut *tx, &uid, &req)
                .await
                .map_err(self.domain_err(&locale))?;

            let norm_key = (parent_id.clone(), tag.name.trim().to_lowercase());
            if let Some(eid) = seen.get(&norm_key) {
                resolver
                    .register(tag.client_ref.as_deref(), eid)
                    .map_err(|e| self.map_err(McpError::BadRequest(e.to_string()), &locale))?;
                result.push(serde_json::json!({"id": eid, "name": tag.name, "existed": true}));
                continue;
            }
            if let Some(eid) = db::tags::find_id_by_name_in_scope(
                &mut *tx,
                &uid,
                &tag.name,
                parent_id.as_deref(),
                None,
            )
            .await
            .map_err(self.db_err(&locale))?
            {
                seen.insert(norm_key, eid.clone());
                resolver
                    .register(tag.client_ref.as_deref(), &eid)
                    .map_err(|e| self.map_err(McpError::BadRequest(e.to_string()), &locale))?;
                result.push(serde_json::json!({"id": eid, "name": tag.name, "existed": true}));
                continue;
            }

            let new_id = Uuid::new_v4().to_string();
            db::tags::insert_in_tx(
                &mut tx,
                &db::tags::InsertTagInput {
                    id: new_id.clone(),
                    user_id: uid.clone(),
                    name: tag.name.clone(),
                    icon: tag.icon.clone(),
                    color: tag.color.clone(),
                    parent_tag_id: parent_id.clone(),
                    tag_type: "tag".to_string(),
                    metadata: None,
                },
            )
            .await
            .map_err(self.db_err(&locale))?;

            seen.insert(norm_key, new_id.clone());
            resolver
                .register(tag.client_ref.as_deref(), &new_id)
                .map_err(|e| self.map_err(McpError::BadRequest(e.to_string()), &locale))?;

            result.push(serde_json::json!({
                "id": new_id,
                "name": tag.name,
                "icon": tag.icon,
                "color": tag.color,
                "parent_tag_id": parent_id,
                "tag_type": "tag",
            }));
        }

        tx.commit().await.map_err(self.sqlx_err(&locale))?;
        self.json_result(result, &locale)
    }

    #[rmcp::tool(
        name = "get_tag_entities",
        description = "mcp-tool-get_tag_entities-desc"
    )]
    pub(crate) async fn get_tag_entities(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<GetTagEntitiesParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;

        if let Some(ref t) = p.entity_type {
            if t != "item" && t != "list" {
                return Err(self.map_err(
                    McpError::BadRequest("invalid_entity_type".to_string()),
                    &locale,
                ));
            }
        }

        let entities = domain::tags::get_entities_by_tag(
            &self.pool,
            &uid,
            &p.tag_id,
            p.entity_type.as_deref(),
        )
        .await
        .map_err(self.domain_err(&locale))?;

        self.json_result(entities, &locale)
    }
}
