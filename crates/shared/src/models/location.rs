use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Country {
    pub id: String,
    pub iso_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Location {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub alias: Option<String>,
    pub region: Option<String>,
    pub address: Option<String>,
    pub location_type: String,
    pub country_id: String,
    pub parent_id: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub created_at: String,
}
