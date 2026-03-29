use serde::{Deserialize, Serialize};
use crate::constants::{FEATURE_QUANTITY, FEATURE_DEADLINES};
use crate::deserializers::{bool_from_number, features_from_json};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListFeature {
    pub name: String,
    #[serde(default)]
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ListType {
    Checklist,
    Zakupy,
    Pakowanie,
    Terminarz,
    Custom,
}

impl ListType {
    pub fn default_features(&self) -> Vec<ListFeature> {
        match self {
            Self::Zakupy | Self::Pakowanie => vec![ListFeature {
                name: FEATURE_QUANTITY.into(),
                config: serde_json::json!({"unit_default": "szt"}),
            }],
            Self::Terminarz => vec![ListFeature {
                name: FEATURE_DEADLINES.into(),
                config: serde_json::json!({"has_start_date": false, "has_deadline": true, "has_hard_deadline": false}),
            }],
            _ => vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DateField {
    StartDate,
    Deadline,
    HardDeadline,
}

impl DateField {
    pub fn column_name(&self) -> &'static str {
        match self {
            Self::StartDate => "start_date",
            Self::Deadline => "deadline",
            Self::HardDeadline => "hard_deadline",
        }
    }

    pub fn time_column_name(&self) -> Option<&'static str> {
        match self {
            Self::StartDate => Some("start_time"),
            Self::Deadline => Some("deadline_time"),
            Self::HardDeadline => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::StartDate => "start",
            Self::Deadline => "deadline",
            Self::HardDeadline => "hard_deadline",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub list_type: ListType,
    pub parent_list_id: Option<String>,
    pub position: i32,
    #[serde(deserialize_with = "bool_from_number")]
    pub archived: bool,
    #[serde(default, deserialize_with = "features_from_json")]
    pub features: Vec<ListFeature>,
    #[serde(default)]
    pub container_id: Option<String>,
    #[serde(default, deserialize_with = "bool_from_number")]
    pub pinned: bool,
    #[serde(default)]
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl List {
    pub fn has_feature(&self, name: &str) -> bool {
        self.features.iter().any(|f| f.name == name)
    }
}
