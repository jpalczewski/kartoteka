use http::request::Parts;
use kartoteka_db as db;
use kartoteka_domain as domain;
use kartoteka_shared::types::ItemChange;
use rmcp::{
    ErrorData,
    handler::server::{tool::Extension, wrapper::Parameters},
    model::CallToolResult,
    tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::McpError;
use crate::server::{KartotekaServer, decode_cursor, encode_cursor, localize_item, today_in_tz};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateItemParams {
    /// Target list ID.
    pub list_id: String,
    pub title: String,
    pub description: Option<String>,
    /// Icon: `lucide:<name>`, `emoji:<e>` or a bare emoji.
    pub icon: Option<String>,
    pub start_date: Option<String>,
    pub deadline: Option<String>,
    pub hard_deadline: Option<String>,
    pub start_time: Option<String>,
    pub deadline_time: Option<String>,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub estimated_duration: Option<i32>,
    /// Repeat interval: Nd, Nw or Nm, e.g. 7d, 2w, 1m.
    /// The next occurrence is created when the item is completed.
    pub recurrence_rule: Option<String>,
    /// fixed (from the scheduled date, default) or floating (from completion date).
    pub recurrence_mode: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateItemParams {
    pub item_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    /// Completing/reopening also moves `status` (done / todo).
    pub completed: Option<bool>,
    /// Icon: `lucide:<name>`, `emoji:<e>` or a bare emoji.
    pub icon: Option<String>,
    pub start_date: Option<String>,
    pub deadline: Option<String>,
    pub hard_deadline: Option<String>,
    pub start_time: Option<String>,
    pub deadline_time: Option<String>,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub estimated_duration: Option<i32>,
    /// Repeat interval: Nd, Nw or Nm, e.g. 7d, 2w, 1m.
    /// The next occurrence is created when the item is completed.
    pub recurrence_rule: Option<String>,
    /// fixed (from the scheduled date, default) or floating (from completion date).
    pub recurrence_mode: Option<String>,
    /// Field names to explicitly clear (set to null).
    /// Valid values: description, icon, start_date, deadline, hard_deadline,
    /// start_time, deadline_time, quantity, actual_quantity, unit, estimated_duration,
    /// recurrence_rule (also clears recurrence_mode), recurrence_mode
    pub clear: Option<Vec<String>>,
}

impl UpdateItemParams {
    fn cleared(&self, name: &str) -> bool {
        self.clear
            .as_deref()
            .is_some_and(|v| v.iter().any(|s| s == name))
    }

    pub fn nullable<T>(val: Option<T>, cleared: bool) -> Option<Option<T>> {
        if cleared { Some(None) } else { val.map(Some) }
    }

    pub fn description_field(&self) -> Option<Option<String>> {
        Self::nullable(self.description.clone(), self.cleared("description"))
    }
    pub fn icon_field(&self) -> Option<Option<String>> {
        Self::nullable(self.icon.clone(), self.cleared("icon"))
    }
    pub fn start_date_field(&self) -> Option<Option<String>> {
        Self::nullable(self.start_date.clone(), self.cleared("start_date"))
    }
    pub fn deadline_field(&self) -> Option<Option<String>> {
        Self::nullable(self.deadline.clone(), self.cleared("deadline"))
    }
    pub fn hard_deadline_field(&self) -> Option<Option<String>> {
        Self::nullable(self.hard_deadline.clone(), self.cleared("hard_deadline"))
    }
    pub fn start_time_field(&self) -> Option<Option<String>> {
        Self::nullable(self.start_time.clone(), self.cleared("start_time"))
    }
    pub fn deadline_time_field(&self) -> Option<Option<String>> {
        Self::nullable(self.deadline_time.clone(), self.cleared("deadline_time"))
    }
    pub fn quantity_field(&self) -> Option<Option<i32>> {
        Self::nullable(self.quantity, self.cleared("quantity"))
    }
    pub fn actual_quantity_field(&self) -> Option<Option<i32>> {
        Self::nullable(self.actual_quantity, self.cleared("actual_quantity"))
    }
    pub fn unit_field(&self) -> Option<Option<String>> {
        Self::nullable(self.unit.clone(), self.cleared("unit"))
    }
    pub fn estimated_duration_field(&self) -> Option<Option<i32>> {
        Self::nullable(self.estimated_duration, self.cleared("estimated_duration"))
    }
    pub fn recurrence_rule_field(&self) -> Option<Option<String>> {
        Self::nullable(
            self.recurrence_rule.clone(),
            self.cleared("recurrence_rule"),
        )
    }
    /// Clearing the rule clears the mode with it, so a cleared rule wins over an
    /// explicit mode: the domain rejects a mode without a rule.
    pub fn recurrence_mode_field(&self) -> Option<Option<String>> {
        let cleared = self.cleared("recurrence_mode") || self.cleared("recurrence_rule");
        Self::nullable(self.recurrence_mode.clone(), cleared)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ToggleItemParams {
    /// Item ID.
    pub item_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetItemStatusParams {
    pub item_id: String,
    /// "todo" | "in_progress" | "done". `done` is the same as completing the
    /// item: unresolved blockers reject it.
    pub status: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateItemsInput {
    pub title: String,
    pub description: Option<String>,
    /// Icon: `lucide:<name>`, `emoji:<e>` or a bare emoji.
    pub icon: Option<String>,
    pub start_date: Option<String>,
    pub deadline: Option<String>,
    pub hard_deadline: Option<String>,
    pub start_time: Option<String>,
    pub deadline_time: Option<String>,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub estimated_duration: Option<i32>,
    /// Repeat interval: Nd, Nw or Nm, e.g. 7d, 2w, 1m.
    /// The next occurrence is created when the item is completed.
    pub recurrence_rule: Option<String>,
    /// fixed (from the scheduled date, default) or floating (from completion date).
    pub recurrence_mode: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateItemsParams {
    /// Target list ID — all items are appended to this list in order.
    pub list_id: String,
    pub items: Vec<CreateItemsInput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetItemParams {
    pub item_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReorderItemsParams {
    pub list_id: String,
    /// Every item id of the list, in the new order. Get them with `list_items`.
    pub item_ids: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveItemParams {
    pub item_id: String,
    /// Target list ID; omit to stay in the item's current list.
    pub list_id: Option<String>,
    /// Place the item right before this item ID; omit to append it at the end.
    pub before_item_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListItemsParams {
    pub list_id: String,
    /// Opaque cursor from previous response; omit for first page
    pub cursor: Option<String>,
    /// Max items to return (1–500, default 100)
    pub limit: Option<u32>,
}

#[tool_router(router = items_router, vis = "pub(crate)")]
impl KartotekaServer {
    #[rmcp::tool(name = "create_item", description = "mcp-tool-create_item-desc")]
    pub(crate) async fn create_item(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<CreateItemParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let req = domain::items::CreateItemRequest {
            title: p.title,
            description: p.description,
            icon: p.icon,
            start_date: p.start_date,
            deadline: p.deadline,
            hard_deadline: p.hard_deadline,
            start_time: p.start_time,
            deadline_time: p.deadline_time,
            quantity: p.quantity,
            actual_quantity: p.actual_quantity,
            unit: p.unit,
            estimated_duration: p.estimated_duration,
            recurrence_rule: p.recurrence_rule,
            recurrence_mode: p.recurrence_mode,
        };
        self.domain_or_existing(
            domain::items::create(&self.pool, &uid, &p.list_id, &req).await,
            &locale,
        )
    }

    #[rmcp::tool(name = "update_item", description = "mcp-tool-update_item-desc")]
    pub(crate) async fn update_item(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<UpdateItemParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let req = domain::items::UpdateItemRequest {
            title: p.title.clone(),
            description: p.description_field(),
            completed: p.completed,
            icon: p.icon_field(),
            quantity: p.quantity_field(),
            actual_quantity: p.actual_quantity_field(),
            unit: p.unit_field(),
            start_date: p.start_date_field(),
            start_time: p.start_time_field(),
            deadline: p.deadline_field(),
            deadline_time: p.deadline_time_field(),
            hard_deadline: p.hard_deadline_field(),
            estimated_duration: p.estimated_duration_field(),
            location_id: None,
            recurrence_rule: p.recurrence_rule_field(),
            recurrence_mode: p.recurrence_mode_field(),
        };
        let item = domain::items::update(&self.pool, &uid, &p.item_id, &req)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(item.map(ItemChange::from), &locale)
    }

    #[rmcp::tool(name = "toggle_item", description = "mcp-tool-toggle_item-desc")]
    #[tracing::instrument(
        skip(self, parts, p),
        fields(action = "mcp_toggle_item", item_id = %p.item_id)
    )]
    pub(crate) async fn toggle_item(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<ToggleItemParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let change = domain::items::toggle_complete(&self.pool, &uid, &p.item_id)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(change.map(ItemChange::from), &locale)
    }

    #[rmcp::tool(
        name = "set_item_status",
        description = "mcp-tool-set_item_status-desc"
    )]
    #[tracing::instrument(skip(self, parts), fields(action = "mcp_set_item_status"))]
    pub(crate) async fn set_item_status(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<SetItemStatusParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let item = domain::items::set_status(&self.pool, &uid, &p.item_id, &p.status)
            .await
            .map_err(self.domain_err(&locale))?
            .ok_or_else(|| {
                self.map_err(
                    McpError::Domain(domain::DomainError::NotFound("item")),
                    &locale,
                )
            })?;
        self.json_result(item, &locale)
    }

    #[rmcp::tool(name = "create_items", description = "mcp-tool-create_items-desc")]
    #[tracing::instrument(skip(self, parts), fields(action = "mcp_create_items"))]
    pub(crate) async fn create_items(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<CreateItemsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;

        if p.items.is_empty() {
            return self.json_result(Vec::<serde_json::Value>::new(), &locale);
        }

        // Validate list ownership + get starting position in one query
        let ctx = db::lists::get_create_item_context(&self.pool, &p.list_id, &uid)
            .await
            .map_err(self.db_err(&locale))?
            .ok_or_else(|| {
                self.map_err(
                    McpError::Domain(domain::DomainError::NotFound("list")),
                    &locale,
                )
            })?;

        // Same rules as the single `create_item` tool (domain::items::create).
        // Everything that needs no DB access is checked up front, before the
        // transaction is opened, and one invalid entry rejects the whole batch.
        let requests: Vec<domain::items::CreateItemRequest> = p
            .items
            .iter()
            .map(|item| domain::items::CreateItemRequest {
                title: item.title.clone(),
                icon: item.icon.clone(),
                description: item.description.clone(),
                quantity: item.quantity,
                actual_quantity: item.actual_quantity,
                unit: item.unit.clone(),
                start_date: item.start_date.clone(),
                start_time: item.start_time.clone(),
                deadline: item.deadline.clone(),
                deadline_time: item.deadline_time.clone(),
                hard_deadline: item.hard_deadline.clone(),
                estimated_duration: item.estimated_duration,
                recurrence_rule: item.recurrence_rule.clone(),
                recurrence_mode: item.recurrence_mode.clone(),
            })
            .collect();
        for req in &requests {
            domain::items::validate_create_item(&ctx.features, req)
                .map_err(self.domain_err(&locale))?;
        }

        let mut tx = self.pool.begin().await.map_err(self.sqlx_err(&locale))?;
        let mut result = Vec::with_capacity(p.items.len());
        let mut next_pos = ctx.next_position as i32;
        let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();

        for req in &requests {
            let norm = req.title.trim().to_lowercase();
            if let Some(eid) = seen.get(&norm) {
                result.push(serde_json::json!({"id": eid, "title": req.title, "existed": true}));
                continue;
            }
            if let Some(eid) =
                db::items::find_id_by_title_in_list(&mut *tx, &p.list_id, &req.title, None)
                    .await
                    .map_err(self.db_err(&locale))?
            {
                seen.insert(norm, eid.clone());
                result.push(serde_json::json!({"id": eid, "title": req.title, "existed": true}));
                continue;
            }
            let input = domain::items::new_item_input(&p.list_id, next_pos, req);
            let inserted = domain::items::insert_item_in(&mut tx, &input)
                .await
                .map_err(self.domain_err(&locale))?;
            seen.insert(norm, inserted.id.clone());
            result.push(serde_json::json!({"id": inserted.id, "title": inserted.title}));
            next_pos += 1;
        }

        tx.commit().await.map_err(self.sqlx_err(&locale))?;
        self.json_result(result, &locale)
    }

    #[rmcp::tool(name = "get_item", description = "mcp-tool-get_item-desc")]
    pub(crate) async fn get_item(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<GetItemParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let tz = Self::timezone(&parts);
        let data = domain::items::get_one(&self.pool, &p.item_id, &uid)
            .await
            .map_err(self.domain_err(&locale))?
            .ok_or_else(|| {
                self.map_err(
                    McpError::Domain(domain::DomainError::NotFound("item")),
                    &locale,
                )
            })?;
        self.json_result(localize_item(data, &tz), &locale)
    }

    #[rmcp::tool(name = "list_items", description = "mcp-tool-list_items-desc")]
    pub(crate) async fn list_items(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<ListItemsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let tz = Self::timezone(&parts);
        let all = domain::items::list_for_list(&self.pool, &p.list_id, &uid)
            .await
            .map_err(self.domain_err(&locale))?;
        let limit = domain::paging::clamp_limit(p.limit);
        let offset = decode_cursor(p.cursor.as_deref());
        let page: Vec<_> = all
            .iter()
            .skip(offset)
            .take(limit as usize)
            .cloned()
            .map(|i| localize_item(i, &tz))
            .collect();
        let next_cursor = if offset + page.len() < all.len() {
            Some(encode_cursor(offset + page.len()))
        } else {
            None
        };
        self.json_result(
            domain::paging::Paged {
                data: page,
                next_cursor,
                limit,
            },
            &locale,
        )
    }

    #[rmcp::tool(name = "list_overdue", description = "mcp-tool-list_overdue-desc")]
    pub(crate) async fn list_overdue(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let tz = Self::timezone(&parts);
        let today = today_in_tz(&tz);
        let data = domain::items::overdue_for(&self.pool, &uid, &today)
            .await
            .map_err(self.domain_err(&locale))?;
        let data: Vec<_> = data.into_iter().map(|i| localize_item(i, &tz)).collect();
        self.json_result(data, &locale)
    }

    #[rmcp::tool(name = "get_today", description = "mcp-tool-get_today-desc")]
    pub(crate) async fn get_today(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let tz = Self::timezone(&parts);
        let today = today_in_tz(&tz);
        let data = domain::items::by_date(&self.pool, &uid, &today)
            .await
            .map_err(self.domain_err(&locale))?;
        let data: Vec<_> = data.into_iter().map(|i| localize_item(i, &tz)).collect();
        self.json_result(data, &locale)
    }

    // ── Ordering tools ──────────────────────────────────────────────────────

    #[rmcp::tool(name = "reorder_items", description = "mcp-tool-reorder_items-desc")]
    #[tracing::instrument(
        skip(self, parts, p),
        fields(
            action = "mcp_reorder_items",
            list_id = %p.list_id,
            count = p.item_ids.len()
        )
    )]
    pub(crate) async fn reorder_items(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<ReorderItemsParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        // A foreign list has an empty scope, which would be indistinguishable from
        // an incomplete order, so check ownership first.
        if domain::lists::get_one(&self.pool, &p.list_id, &uid)
            .await
            .map_err(self.domain_err(&locale))?
            .is_none()
        {
            return Err(self.map_err(
                McpError::Domain(domain::DomainError::NotFound("list")),
                &locale,
            ));
        }
        let scope = db::items::ordered_ids_in_list(&self.pool, &uid, &p.list_id)
            .await
            .map_err(self.db_err(&locale))?;
        // Foreign and repeated ids are reported as such, before the completeness
        // check, so the client learns what is actually wrong.
        domain::DomainError::ensure_ids_in_scope(&scope, &p.item_ids, "item")
            .map_err(self.domain_err(&locale))?;
        // `items::reorder` writes positions 0..n for exactly the ids it receives,
        // so a partial order would collide with the positions of the items it
        // left out. The full set is required here and not in the domain layer:
        // the UI legitimately reorders only the items it has on screen.
        if p.item_ids.len() != scope.len() {
            return Err(self.map_err(
                McpError::Domain(domain::DomainError::Validation("incomplete_order")),
                &locale,
            ));
        }
        domain::items::reorder(&self.pool, &uid, &p.list_id, &p.item_ids)
            .await
            .map_err(self.domain_err(&locale))?;
        self.json_result(serde_json::json!({"reordered": p.item_ids.len()}), &locale)
    }

    #[rmcp::tool(name = "move_item", description = "mcp-tool-move_item-desc")]
    #[tracing::instrument(
        skip(self, parts, p),
        fields(action = "mcp_move_item", item_id = %p.item_id)
    )]
    pub(crate) async fn move_item(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(p): Parameters<MoveItemParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let (uid, locale) = self.auth(&parts)?;
        let tz = Self::timezone(&parts);
        let current = domain::items::get_one(&self.pool, &p.item_id, &uid)
            .await
            .map_err(self.domain_err(&locale))?
            .ok_or_else(|| {
                self.map_err(
                    McpError::Domain(domain::DomainError::NotFound("item")),
                    &locale,
                )
            })?;
        let target_list_id = p.list_id.unwrap_or(current.list_id);
        domain::items::place(
            &self.pool,
            &uid,
            &p.item_id,
            &target_list_id,
            p.before_item_id.as_deref(),
        )
        .await
        .map_err(self.domain_err(&locale))?;
        let moved = domain::items::get_one(&self.pool, &p.item_id, &uid)
            .await
            .map_err(self.domain_err(&locale))?
            .ok_or_else(|| {
                self.map_err(
                    McpError::Domain(domain::DomainError::NotFound("item")),
                    &locale,
                )
            })?;
        self.json_result(localize_item(moved, &tz), &locale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::test_support::{
        insert_test_list, item_json, multi_conn_pool, parts_for_user, test_server,
    };
    use kartoteka_db::test_helpers::create_test_user;
    use serde_json::json;

    /// Mirrors what `update_item` hands to `domain::items::update`.
    fn update_request(p: &UpdateItemParams) -> kartoteka_domain::items::UpdateItemRequest {
        kartoteka_domain::items::UpdateItemRequest {
            title: p.title.clone(),
            description: p.description_field(),
            completed: p.completed,
            icon: p.icon_field(),
            quantity: p.quantity_field(),
            actual_quantity: p.actual_quantity_field(),
            unit: p.unit_field(),
            start_date: p.start_date_field(),
            start_time: p.start_time_field(),
            deadline: p.deadline_field(),
            deadline_time: p.deadline_time_field(),
            hard_deadline: p.hard_deadline_field(),
            estimated_duration: p.estimated_duration_field(),
            location_id: None,
            recurrence_rule: p.recurrence_rule_field(),
            recurrence_mode: p.recurrence_mode_field(),
        }
    }

    #[test]
    fn create_item_params_map_recurrence_fields_to_request() {
        let p: CreateItemParams = serde_json::from_value(json!({
            "list_id": "list-1",
            "title": "Water plants",
            "recurrence_rule": "7d",
            "recurrence_mode": "floating",
        }))
        .unwrap();
        let req = kartoteka_domain::items::CreateItemRequest {
            title: p.title,
            description: p.description,
            start_date: p.start_date,
            deadline: p.deadline,
            hard_deadline: p.hard_deadline,
            start_time: p.start_time,
            deadline_time: p.deadline_time,
            quantity: p.quantity,
            actual_quantity: p.actual_quantity,
            unit: p.unit,
            estimated_duration: p.estimated_duration,
            recurrence_rule: p.recurrence_rule,
            recurrence_mode: p.recurrence_mode,
            icon: p.icon,
        };
        assert_eq!(req.recurrence_rule.as_deref(), Some("7d"));
        assert_eq!(req.recurrence_mode.as_deref(), Some("floating"));
    }

    #[test]
    fn create_items_input_maps_recurrence_fields_to_request() {
        let input: CreateItemsInput = serde_json::from_value(json!({
            "title": "Pay rent",
            "recurrence_rule": "1m",
            "recurrence_mode": "fixed",
        }))
        .unwrap();
        let req = kartoteka_domain::items::CreateItemRequest {
            title: input.title,
            description: input.description,
            start_date: input.start_date,
            deadline: input.deadline,
            hard_deadline: input.hard_deadline,
            start_time: input.start_time,
            deadline_time: input.deadline_time,
            quantity: input.quantity,
            actual_quantity: input.actual_quantity,
            unit: input.unit,
            estimated_duration: input.estimated_duration,
            recurrence_rule: input.recurrence_rule,
            recurrence_mode: input.recurrence_mode,
            icon: input.icon,
        };
        assert_eq!(req.recurrence_rule.as_deref(), Some("1m"));
        assert_eq!(req.recurrence_mode.as_deref(), Some("fixed"));
    }

    #[test]
    fn update_item_params_map_recurrence_fields_to_request() {
        let p: UpdateItemParams = serde_json::from_value(json!({
            "item_id": "item-1",
            "recurrence_rule": "2w",
            "recurrence_mode": "floating",
        }))
        .unwrap();
        let req = update_request(&p);
        assert_eq!(req.recurrence_rule, Some(Some("2w".to_owned())));
        assert_eq!(req.recurrence_mode, Some(Some("floating".to_owned())));
    }

    #[test]
    fn update_item_params_without_recurrence_leave_fields_unset() {
        let p: UpdateItemParams =
            serde_json::from_value(json!({"item_id": "item-1", "title": "Renamed"})).unwrap();
        let req = update_request(&p);
        assert_eq!(req.recurrence_rule, None);
        assert_eq!(req.recurrence_mode, None);
    }

    #[test]
    fn clearing_recurrence_rule_clears_the_mode_too() {
        let p: UpdateItemParams = serde_json::from_value(json!({
            "item_id": "item-1",
            "clear": ["recurrence_rule"],
        }))
        .unwrap();
        let req = update_request(&p);
        assert_eq!(req.recurrence_rule, Some(None));
        assert_eq!(req.recurrence_mode, Some(None));
    }

    #[test]
    fn explicit_recurrence_mode_is_kept_when_given() {
        let p: UpdateItemParams = serde_json::from_value(json!({
            "item_id": "item-1",
            "recurrence_rule": "3d",
            "recurrence_mode": "floating",
            "clear": ["deadline"],
        }))
        .unwrap();
        let req = update_request(&p);
        assert_eq!(req.recurrence_rule, Some(Some("3d".to_owned())));
        assert_eq!(req.recurrence_mode, Some(Some("floating".to_owned())));
    }

    #[test]
    fn clearing_recurrence_mode_keeps_the_rule() {
        let p: UpdateItemParams = serde_json::from_value(json!({
            "item_id": "item-1",
            "recurrence_rule": "3d",
            "clear": ["recurrence_mode"],
        }))
        .unwrap();
        let req = update_request(&p);
        assert_eq!(req.recurrence_rule, Some(Some("3d".to_owned())));
        assert_eq!(req.recurrence_mode, Some(None));
    }

    // ── reorder_items / move_item (#385) ────────────────────────────────────

    /// Insert an item row directly, so the positions under test are explicit.
    async fn insert_item(
        server: &KartotekaServer,
        list_id: &str,
        title: &str,
        position: i32,
    ) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO items (id, list_id, title, position) VALUES (?, ?, ?, ?)")
            .bind(&id)
            .bind(list_id)
            .bind(title)
            .bind(position)
            .execute(&server.pool)
            .await
            .unwrap();
        id
    }

    /// `<data.error>/<data.reason>` of a mapped error, for compact assertions.
    fn error_reason(err: &ErrorData) -> String {
        let data = err.data.as_ref().expect("mapped errors carry data");
        let field = |key: &str| data[key].as_str().unwrap_or("").to_owned();
        format!("{}/{}", field("error"), field("reason"))
    }

    #[tokio::test]
    async fn reorder_items_persists_the_full_order() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &uid).await;
        let server = test_server(pool);
        let a = insert_item(&server, &list_id, "A", 0).await;
        let b = insert_item(&server, &list_id, "B", 1).await;
        let c = insert_item(&server, &list_id, "C", 2).await;

        let result = server
            .reorder_items(
                Extension(parts_for_user(&uid)),
                Parameters(ReorderItemsParams {
                    list_id: list_id.clone(),
                    item_ids: vec![c.clone(), a.clone(), b.clone()],
                }),
            )
            .await
            .unwrap();

        assert_eq!(item_json(&result).await, json!({"reordered": 3}));
        let ordered = db::items::ordered_ids_in_list(&server.pool, &uid, &list_id)
            .await
            .unwrap();
        assert_eq!(ordered, vec![c, a, b]);
    }

    #[tokio::test]
    async fn reorder_items_rejects_a_partial_order() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &uid).await;
        let server = test_server(pool);
        let a = insert_item(&server, &list_id, "A", 0).await;
        let b = insert_item(&server, &list_id, "B", 1).await;
        let c = insert_item(&server, &list_id, "C", 2).await;

        let err = server
            .reorder_items(
                Extension(parts_for_user(&uid)),
                Parameters(ReorderItemsParams {
                    list_id: list_id.clone(),
                    item_ids: vec![c.clone(), a.clone()],
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(err.code.0, -32602, "unexpected code: {err:?}");
        assert_eq!(error_reason(&err), "validation/incomplete_order");
        // Nothing was written: the positions still describe the old order.
        let ordered = db::items::ordered_ids_in_list(&server.pool, &uid, &list_id)
            .await
            .unwrap();
        assert_eq!(ordered, vec![a, b, c]);
    }

    #[tokio::test]
    async fn reorder_items_rejects_an_item_from_another_list() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &uid).await;
        let other_list = insert_test_list(&pool, &uid).await;
        let server = test_server(pool);
        let a = insert_item(&server, &list_id, "A", 0).await;
        let b = insert_item(&server, &list_id, "B", 1).await;
        let elsewhere = insert_item(&server, &other_list, "X", 0).await;

        let err = server
            .reorder_items(
                Extension(parts_for_user(&uid)),
                Parameters(ReorderItemsParams {
                    list_id: list_id.clone(),
                    item_ids: vec![elsewhere, a.clone(), b.clone()],
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(err.code.0, -32602, "unexpected code: {err:?}");
        assert_eq!(error_reason(&err), "not_found/");
        assert_eq!(
            err.data.as_ref().unwrap()["entity"],
            "item",
            "the foreign id is reported as a missing item"
        );
        let ordered = db::items::ordered_ids_in_list(&server.pool, &uid, &list_id)
            .await
            .unwrap();
        assert_eq!(ordered, vec![a, b]);
    }

    #[tokio::test]
    async fn reorder_items_rejects_a_foreign_list() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let other_uid = create_test_user(&pool).await;
        let foreign_list = insert_test_list(&pool, &other_uid).await;
        let server = test_server(pool);
        let foreign_item = insert_item(&server, &foreign_list, "X", 0).await;

        let err = server
            .reorder_items(
                Extension(parts_for_user(&uid)),
                Parameters(ReorderItemsParams {
                    list_id: foreign_list.clone(),
                    item_ids: vec![foreign_item],
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(err.code.0, -32602, "unexpected code: {err:?}");
        assert_eq!(err.data.as_ref().unwrap()["entity"], "list");
    }

    #[tokio::test]
    async fn move_item_appends_to_the_target_list() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let source = insert_test_list(&pool, &uid).await;
        let target = insert_test_list(&pool, &uid).await;
        let server = test_server(pool);
        let moving = insert_item(&server, &source, "Milk", 0).await;
        let first = insert_item(&server, &target, "Bread", 0).await;
        let second = insert_item(&server, &target, "Eggs", 1).await;

        let result = server
            .move_item(
                Extension(parts_for_user(&uid)),
                Parameters(MoveItemParams {
                    item_id: moving.clone(),
                    list_id: Some(target.clone()),
                    before_item_id: None,
                }),
            )
            .await
            .unwrap();

        let json = item_json(&result).await;
        assert_eq!(json["id"], moving.as_str());
        assert_eq!(json["list_id"], target.as_str());
        assert_eq!(json["position"], 2);
        let ordered = db::items::ordered_ids_in_list(&server.pool, &uid, &target)
            .await
            .unwrap();
        assert_eq!(ordered, vec![first, second, moving.clone()]);
        assert!(
            db::items::ordered_ids_in_list(&server.pool, &uid, &source)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn move_item_inserts_before_the_given_item() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let source = insert_test_list(&pool, &uid).await;
        let target = insert_test_list(&pool, &uid).await;
        let server = test_server(pool);
        let moving = insert_item(&server, &source, "Milk", 0).await;
        let first = insert_item(&server, &target, "Bread", 0).await;
        let second = insert_item(&server, &target, "Eggs", 1).await;

        let result = server
            .move_item(
                Extension(parts_for_user(&uid)),
                Parameters(MoveItemParams {
                    item_id: moving.clone(),
                    list_id: Some(target.clone()),
                    before_item_id: Some(second.clone()),
                }),
            )
            .await
            .unwrap();

        assert_eq!(item_json(&result).await["position"], 1);
        let ordered = db::items::ordered_ids_in_list(&server.pool, &uid, &target)
            .await
            .unwrap();
        assert_eq!(ordered, vec![first, moving, second]);
    }

    #[tokio::test]
    async fn move_item_without_list_id_stays_in_the_current_list() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &uid).await;
        let server = test_server(pool);
        let a = insert_item(&server, &list_id, "A", 0).await;
        let b = insert_item(&server, &list_id, "B", 1).await;
        let c = insert_item(&server, &list_id, "C", 2).await;

        let result = server
            .move_item(
                Extension(parts_for_user(&uid)),
                Parameters(MoveItemParams {
                    item_id: c.clone(),
                    list_id: None,
                    before_item_id: Some(a.clone()),
                }),
            )
            .await
            .unwrap();

        let json = item_json(&result).await;
        assert_eq!(json["list_id"], list_id.as_str());
        assert_eq!(json["position"], 0);
        let ordered = db::items::ordered_ids_in_list(&server.pool, &uid, &list_id)
            .await
            .unwrap();
        assert_eq!(ordered, vec![c, a, b]);
    }

    #[tokio::test]
    async fn move_item_to_a_foreign_list_is_not_found() {
        let pool = multi_conn_pool().await;
        let uid = create_test_user(&pool).await;
        let other_uid = create_test_user(&pool).await;
        let source = insert_test_list(&pool, &uid).await;
        let foreign_list = insert_test_list(&pool, &other_uid).await;
        let server = test_server(pool);
        let moving = insert_item(&server, &source, "Milk", 0).await;

        let err = server
            .move_item(
                Extension(parts_for_user(&uid)),
                Parameters(MoveItemParams {
                    item_id: moving.clone(),
                    list_id: Some(foreign_list),
                    before_item_id: None,
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(err.code.0, -32602, "unexpected code: {err:?}");
        assert_eq!(err.data.as_ref().unwrap()["entity"], "list");
        let (stored_list, position) =
            sqlx::query_as::<_, (String, i32)>("SELECT list_id, position FROM items WHERE id = ?")
                .bind(&moving)
                .fetch_one(&server.pool)
                .await
                .unwrap();
        assert_eq!((stored_list, position), (source, 0));
    }
}
