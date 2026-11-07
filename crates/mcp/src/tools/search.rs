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
pub struct SearchItemsParams {
    pub query: String,
}

#[tool_router(router = search_router, vis = "pub(crate)")]
impl KartotekaServer {
    #[rmcp::tool(name = "search_items", description = "mcp-tool-search_items-desc")]
    pub(crate) async fn search_items(
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
}
