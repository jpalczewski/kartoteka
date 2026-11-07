use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateItemParams {
    /// Target list ID.
    pub list_id: String,
    pub title: String,
    pub description: Option<String>,
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
    pub completed: Option<bool>,
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
    /// Valid values: description, start_date, deadline, hard_deadline,
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

// ── toggle_item ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ToggleItemParams {
    /// Item ID.
    pub item_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateListParams {
    pub name: String,
    /// List type: "checklist" (default), "shopping", "schedule", "log", "notes"
    pub list_type: Option<String>,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub container_id: Option<String>,
    pub parent_list_id: Option<String>,
    /// Feature names to enable, e.g. ["quantity", "deadline"]
    pub features: Option<Vec<String>>,
}

// ── create_container ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateContainerParams {
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    /// null/omit = folder; "active" | "done" | "paused" = project
    pub status: Option<String>,
    pub parent_container_id: Option<String>,
}

// ── create_items (batch) ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateItemsInput {
    pub title: String,
    pub description: Option<String>,
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

// ── create_lists (batch) ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateListsInput {
    pub name: String,
    /// List type: "checklist" (default), "shopping", "schedule", "log", "notes"
    pub list_type: Option<String>,
    pub icon: Option<String>,
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

// ── create_containers (batch) ─────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateContainersInput {
    pub name: String,
    pub icon: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Mirrors what `update_item` hands to `domain::items::update`.
    fn update_request(p: &UpdateItemParams) -> kartoteka_domain::items::UpdateItemRequest {
        kartoteka_domain::items::UpdateItemRequest {
            title: p.title.clone(),
            description: p.description_field(),
            completed: p.completed,
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
}
