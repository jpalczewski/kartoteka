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

use crate::server::KartotekaServer;

#[derive(Deserialize, JsonSchema)]
pub struct AddCommentParams {
    /// "item", "list", or "container"
    pub entity_type: String,
    pub entity_id: String,
    pub content: String,
    /// Display name shown as comment author (e.g. "Claude"). Omit to attribute to the logged-in user.
    pub author_name: Option<String>,
}

#[tool_router(router = comments_router, vis = "pub(crate)")]
impl KartotekaServer {
    #[rmcp::tool(name = "add_comment", description = "mcp-tool-add_comment-desc")]
    pub(crate) async fn add_comment(
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
            // Comments written through MCP come from an AI assistant, not from
            // the user typing: the UI renders `assistant` with an "AI" badge and
            // falls back to the author name. `author_name` is kept as given.
            "assistant",
            p.author_name.as_deref(),
        )
        .await
        .map_err(self.domain_err(&locale))?;
        self.json_result(comment, &locale)
    }
}
