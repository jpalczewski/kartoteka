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

    pub fn nullable<T>(val: Option<T>, _name: &str, cleared: bool) -> Option<Option<T>> {
        if cleared { Some(None) } else { val.map(Some) }
    }

    pub fn description_field(&self) -> Option<Option<String>> {
        Self::nullable(
            self.description.clone(),
            "description",
            self.cleared("description"),
        )
    }
    pub fn start_date_field(&self) -> Option<Option<String>> {
        Self::nullable(
            self.start_date.clone(),
            "start_date",
            self.cleared("start_date"),
        )
    }
    pub fn deadline_field(&self) -> Option<Option<String>> {
        Self::nullable(self.deadline.clone(), "deadline", self.cleared("deadline"))
    }
    pub fn hard_deadline_field(&self) -> Option<Option<String>> {
        Self::nullable(
            self.hard_deadline.clone(),
            "hard_deadline",
            self.cleared("hard_deadline"),
        )
    }
    pub fn start_time_field(&self) -> Option<Option<String>> {
        Self::nullable(
            self.start_time.clone(),
            "start_time",
            self.cleared("start_time"),
        )
    }
    pub fn deadline_time_field(&self) -> Option<Option<String>> {
        Self::nullable(
            self.deadline_time.clone(),
            "deadline_time",
            self.cleared("deadline_time"),
        )
    }
    pub fn quantity_field(&self) -> Option<Option<i32>> {
        Self::nullable(self.quantity, "quantity", self.cleared("quantity"))
    }
    pub fn actual_quantity_field(&self) -> Option<Option<i32>> {
        Self::nullable(
            self.actual_quantity,
            "actual_quantity",
            self.cleared("actual_quantity"),
        )
    }
    pub fn unit_field(&self) -> Option<Option<String>> {
        Self::nullable(self.unit.clone(), "unit", self.cleared("unit"))
    }
    pub fn estimated_duration_field(&self) -> Option<Option<i32>> {
        Self::nullable(
            self.estimated_duration,
            "estimated_duration",
            self.cleared("estimated_duration"),
        )
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
