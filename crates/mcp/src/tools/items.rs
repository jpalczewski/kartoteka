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
    /// Field names to explicitly clear (set to null).
    /// Valid values: description, start_date, deadline, hard_deadline,
    /// start_time, deadline_time, quantity, actual_quantity, unit, estimated_duration
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
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateListParams {
    pub name: String,
    /// List type: "checklist" (default), "shopping", "habit", or "custom"
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
