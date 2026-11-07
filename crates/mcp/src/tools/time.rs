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
pub struct StartTimerParams {
    pub item_id: Option<String>,
    /// "stopwatch" (default) or "pomodoro". Breaks are tracked by the client;
    /// the server only records which mode the work session belongs to.
    pub mode: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct LogTimeParams {
    pub item_id: Option<String>,
    /// Format: "YYYY-MM-DD HH:MM:SS"
    pub started_at: String,
    /// Format: "YYYY-MM-DD HH:MM:SS"
    pub ended_at: String,
    pub description: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetTimeSummaryParams {
    /// "item", "list" or "container". With `entity_id`, returns the recursive
    /// tracked-time total for that entity; omit both to list recent entries.
    pub entity_type: Option<String>,
    /// id of the item/list/container named by `entity_type`.
    pub entity_id: Option<String>,
}

#[tool_router(router = time_router, vis = "pub(crate)")]
impl KartotekaServer {
    #[rmcp::tool(name = "start_timer", description = "mcp-tool-start_timer-desc")]
    pub(crate) async fn start_timer(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<StartTimerParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        // Omitted mode = stopwatch; an unknown mode is a validation error.
        let mode = match p.mode.as_deref() {
            Some(m) => {
                domain::time_entries::TimerMode::parse(m).map_err(self.domain_err(&locale))?
            }
            None => domain::time_entries::TimerMode::default(),
        };
        let entry =
            domain::time_entries::start_with_mode(&self.pool, &uid, p.item_id.as_deref(), mode)
                .await
                .map_err(self.domain_err(&locale))?;
        self.json_result(entry, &locale)
    }

    #[rmcp::tool(name = "stop_timer", description = "mcp-tool-stop_timer-desc")]
    pub(crate) async fn stop_timer(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let entry = domain::time_entries::stop(&self.pool, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(entry, &locale)
    }

    #[rmcp::tool(name = "log_time", description = "mcp-tool-log_time-desc")]
    pub(crate) async fn log_time(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<LogTimeParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let entry = domain::time_entries::log_manual(
            &self.pool,
            &uid,
            p.item_id.as_deref(),
            &p.started_at,
            &p.ended_at,
            p.description.as_deref(),
        )
        .await
        .map_err(self.domain_err(&locale))?;
        self.json_result(entry, &locale)
    }

    #[rmcp::tool(
        name = "get_active_timer",
        description = "mcp-tool-get_active_timer-desc"
    )]
    pub(crate) async fn get_active_timer(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let data = domain::time_entries::get_active(&self.pool, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(data, &locale)
    }

    #[rmcp::tool(
        name = "get_time_summary",
        description = "mcp-tool-get_time_summary-desc"
    )]
    pub(crate) async fn get_time_summary(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<GetTimeSummaryParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        if let (Some(entity_type), Some(entity_id)) = (p.entity_type, p.entity_id) {
            let total = self
                .entity_time_total(&uid, &entity_type, &entity_id)
                .await
                .map_err(self.domain_err(&locale))?;
            return self.json_result(
                json!({
                    "entity_type": entity_type,
                    "entity_id": entity_id,
                    "seconds": total.seconds,
                    "running_since": total.running_since,
                    "human": domain::time_entries::format_h_mm(total.seconds),
                }),
                &locale,
            );
        }
        let data = domain::time_entries::list_all_for_user(&self.pool, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(data, &locale)
    }

    /// Recursive tracked-time total for one entity (`item`, `list` or
    /// `container`). Kept as a helper so the tool body stays small and the
    /// entity-type dispatch lives in one place.
    async fn entity_time_total(
        &self,
        uid: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<domain::time_entries::TimeTotal, domain::DomainError> {
        match entity_type {
            "item" => domain::time_entries::total_for_item(&self.pool, uid, entity_id).await,
            "list" => domain::time_entries::total_for_list(&self.pool, uid, entity_id).await,
            "container" => {
                domain::time_entries::total_for_container(&self.pool, uid, entity_id).await
            }
            _ => Err(domain::DomainError::Validation("invalid_entity_type")),
        }
    }
}
