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
pub struct CreateListFromTemplateParams {
    pub template_id: String,
    pub list_name: String,
    /// "checklist" (default), "shopping", "schedule", "log", "notes"
    pub list_type: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct SaveAsTemplateParams {
    pub list_id: String,
    pub template_name: String,
}

#[tool_router(router = templates_router, vis = "pub(crate)")]
impl KartotekaServer {
    #[rmcp::tool(
        name = "create_list_from_template",
        description = "mcp-tool-create_list_from_template-desc"
    )]
    pub(crate) async fn create_list_from_template(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<CreateListFromTemplateParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let list_type = p.list_type.as_deref().unwrap_or("checklist");
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
    pub(crate) async fn save_as_template(
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

    #[rmcp::tool(name = "list_templates", description = "mcp-tool-list_templates-desc")]
    pub(crate) async fn list_templates(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let data = domain::templates::list(&self.pool, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(data, &locale)
    }
}
