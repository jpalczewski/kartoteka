use http::request::Parts;
use kartoteka_domain as domain;
use rmcp::{
    ErrorData,
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::server::KartotekaServer;

#[derive(Deserialize, JsonSchema)]
pub struct AddRelationParams {
    pub from_type: String,
    pub from_id: String,
    pub to_type: String,
    pub to_id: String,
    /// "blocks" or "relates_to"
    pub relation_type: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct RemoveRelationParams {
    pub relation_id: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetRelationsParams {
    /// "item", "list" or "container"
    pub entity_type: String,
    pub entity_id: String,
}

#[tool_router(router = relations_router, vis = "pub(crate)")]
impl KartotekaServer {
    #[rmcp::tool(name = "add_relation", description = "mcp-tool-add_relation-desc")]
    pub(crate) async fn add_relation(
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
    pub(crate) async fn remove_relation(
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

    #[rmcp::tool(name = "get_relations", description = "mcp-tool-get_relations-desc")]
    pub(crate) async fn get_relations(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<GetRelationsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let rels =
            domain::relations::get_for_entity(&self.pool, &uid, &p.entity_type, &p.entity_id)
                .await
                .map_err(self.domain_err(&locale))?;
        self.json_result(rels, &locale)
    }
}
