use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
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
            Self::StartDate => "start_date",
            Self::Deadline => "deadline",
            Self::HardDeadline => "hard_deadline",
        }
    }
}
