use serde::{Deserialize, Deserializer, Serialize};

fn bool_from_number<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    match v {
        serde_json::Value::Bool(b) => Ok(b),
        serde_json::Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0) != 0.0),
        _ => Ok(false),
    }
}

// === Domain types ===

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListType {
    Shopping,
    Packing,
    Project,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    pub id: String,
    pub name: String,
    pub list_type: ListType,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub description: Option<String>,
    #[serde(deserialize_with = "bool_from_number")]
    pub completed: bool,
    pub position: i32,
    pub created_at: String,
    pub updated_at: String,
}

// === Request DTOs ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateListRequest {
    pub name: String,
    pub list_type: ListType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateListRequest {
    pub name: Option<String>,
    pub list_type: Option<ListType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateItemRequest {
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateItemRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub completed: Option<bool>,
    pub position: Option<i32>,
}
