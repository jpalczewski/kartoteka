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
pub struct AssignTimeEntryParams {
    /// id of the inbox entry to assign
    pub entry_id: String,
    /// id of the item the entry should belong to
    pub item_id: String,
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

    #[rmcp::tool(
        name = "list_time_inbox",
        description = "mcp-tool-list_time_inbox-desc"
    )]
    pub(crate) async fn list_time_inbox(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let data = domain::time_entries::list_inbox(&self.pool, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(data, &locale)
    }

    #[rmcp::tool(
        name = "assign_time_entry",
        description = "mcp-tool-assign_time_entry-desc"
    )]
    pub(crate) async fn assign_time_entry(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<AssignTimeEntryParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let entry = domain::time_entries::assign(&self.pool, &uid, &p.entry_id, &p.item_id)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(entry, &locale)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::test_support::*;
    use kartoteka_db::test_helpers::create_test_user;

    /// Start an inbox timer through the tool (no `item_id`), the way an agent
    /// records work it has not attached to an item yet.
    async fn start_inbox_entry(server: &KartotekaServer, uid: &str) {
        server
            .start_timer(
                Extension(parts_for_user(uid)),
                Parameters(StartTimerParams {
                    item_id: None,
                    mode: None,
                }),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn list_time_inbox_returns_only_own_unassigned_entries() {
        let pool = multi_conn_pool().await;
        let owner = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let list_id = insert_test_list_with_features(&pool, &owner, &["time_tracking"]).await;
        let item_id = kartoteka_db::test_helpers::create_test_item(&pool, &list_id).await;
        let server = test_server(pool);

        // One entry attached to an item, one left in the inbox.
        server
            .log_time(
                Extension(parts_for_user(&owner)),
                Parameters(LogTimeParams {
                    item_id: Some(item_id),
                    started_at: "2026-01-01 10:00:00".to_owned(),
                    ended_at: "2026-01-01 11:00:00".to_owned(),
                    description: None,
                }),
            )
            .await
            .unwrap();
        start_inbox_entry(&server, &owner).await;
        start_inbox_entry(&server, &other).await;

        let result = server
            .list_time_inbox(Extension(parts_for_user(&owner)))
            .await
            .unwrap();

        let data = item_json(&result).await;
        let entries = data.as_array().expect("a list of time entries");
        assert_eq!(entries.len(), 1, "only the owner's unassigned entry");
        assert!(entries[0]["item_id"].is_null());
        assert_eq!(entries[0]["user_id"], owner);
    }

    #[tokio::test]
    async fn assign_time_entry_attaches_entry_and_empties_the_inbox() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = insert_test_list_with_features(&pool, &uid, &["time_tracking"]).await;
        let item_id = kartoteka_db::test_helpers::create_test_item(&pool, &list_id).await;
        let server = test_server(pool);
        start_inbox_entry(&server, &uid).await;

        let inbox = item_json(
            &server
                .list_time_inbox(Extension(parts_for_user(&uid)))
                .await
                .unwrap(),
        )
        .await;
        let entry_id = inbox[0]["id"].as_str().unwrap().to_owned();

        let result = server
            .assign_time_entry(
                Extension(parts_for_user(&uid)),
                Parameters(AssignTimeEntryParams {
                    entry_id,
                    item_id: item_id.clone(),
                }),
            )
            .await
            .unwrap();

        let assigned = item_json(&result).await;
        assert_eq!(assigned["item_id"], item_id);

        let inbox_after = item_json(
            &server
                .list_time_inbox(Extension(parts_for_user(&uid)))
                .await
                .unwrap(),
        )
        .await;
        assert!(inbox_after.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn assign_time_entry_requires_time_tracking_on_the_target_list() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &uid).await;
        let item_id = kartoteka_db::test_helpers::create_test_item(&pool, &list_id).await;
        let server = test_server(pool);
        start_inbox_entry(&server, &uid).await;

        let inbox = item_json(
            &server
                .list_time_inbox(Extension(parts_for_user(&uid)))
                .await
                .unwrap(),
        )
        .await;
        let entry_id = inbox[0]["id"].as_str().unwrap().to_owned();

        let err = server
            .assign_time_entry(
                Extension(parts_for_user(&uid)),
                Parameters(AssignTimeEntryParams { entry_id, item_id }),
            )
            .await
            .unwrap_err();

        assert_eq!(err.code.0, -32602);
        let data = err.data.as_ref().unwrap();
        assert_eq!(data["error"], "feature_required");
        assert_eq!(data["feature"], "time_tracking");
    }

    #[tokio::test]
    async fn assign_time_entry_to_a_foreign_item_is_forbidden() {
        let pool = multi_conn_pool().await;
        let owner = create_test_user(&pool).await;
        let attacker = create_test_user(&pool).await;
        let list_id = insert_test_list_with_features(&pool, &owner, &["time_tracking"]).await;
        let item_id = kartoteka_db::test_helpers::create_test_item(&pool, &list_id).await;
        let server = test_server(pool);
        start_inbox_entry(&server, &attacker).await;

        let inbox = item_json(
            &server
                .list_time_inbox(Extension(parts_for_user(&attacker)))
                .await
                .unwrap(),
        )
        .await;
        let entry_id = inbox[0]["id"].as_str().unwrap().to_owned();

        let err = server
            .assign_time_entry(
                Extension(parts_for_user(&attacker)),
                Parameters(AssignTimeEntryParams { entry_id, item_id }),
            )
            .await
            .unwrap_err();

        assert_eq!(err.code.0, -32600);
        assert_eq!(err.data.as_ref().unwrap()["error"], "forbidden");
    }

    #[tokio::test]
    async fn assign_time_entry_with_unknown_entry_is_not_found() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = insert_test_list_with_features(&pool, &uid, &["time_tracking"]).await;
        let item_id = kartoteka_db::test_helpers::create_test_item(&pool, &list_id).await;
        let server = test_server(pool);

        let err = server
            .assign_time_entry(
                Extension(parts_for_user(&uid)),
                Parameters(AssignTimeEntryParams {
                    entry_id: "missing-entry".to_owned(),
                    item_id,
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(err.code.0, -32602);
        let data = err.data.as_ref().unwrap();
        assert_eq!(data["error"], "not_found");
        assert_eq!(data["entity"], "time_entry");
    }
}
