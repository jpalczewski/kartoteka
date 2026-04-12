use chrono::NaiveDate;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// Flexible date with day, week, or month precision.
///
/// SQLite storage: TEXT column.
/// - 10 chars "YYYY-MM-DD" → Day
/// - 8 chars "YYYY-Wnn" → Week (ISO week)
/// - 7 chars "YYYY-MM" → Month
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FlexDate {
    Day(NaiveDate),
    Week(u16, u8),
    Month(u16, u8),
}

impl FlexDate {
    /// First day of the period.
    pub fn start(&self) -> NaiveDate {
        match self {
            FlexDate::Day(d) => *d,
            FlexDate::Week(year, week) => {
                NaiveDate::from_isoywd_opt(i32::from(*year), u32::from(*week), chrono::Weekday::Mon)
                    .expect("valid ISO week")
            }
            FlexDate::Month(year, month) => {
                NaiveDate::from_ymd_opt(i32::from(*year), u32::from(*month), 1)
                    .expect("valid month")
            }
        }
    }

    /// Last day of the period.
    pub fn end(&self) -> NaiveDate {
        match self {
            FlexDate::Day(d) => *d,
            FlexDate::Week(year, week) => {
                NaiveDate::from_isoywd_opt(i32::from(*year), u32::from(*week), chrono::Weekday::Sun)
                    .expect("valid ISO week")
            }
            FlexDate::Month(year, month) => {
                let (y, m) = if *month == 12 {
                    (i32::from(*year) + 1, 1)
                } else {
                    (i32::from(*year), u32::from(*month) + 1)
                };
                NaiveDate::from_ymd_opt(y, m, 1)
                    .expect("valid date")
                    .pred_opt()
                    .expect("valid pred")
            }
        }
    }

    /// True if not day-level precision.
    pub fn is_fuzzy(&self) -> bool {
        !matches!(self, FlexDate::Day(_))
    }

    /// Check if a specific day falls within this date's range.
    pub fn matches_day(&self, day: NaiveDate) -> bool {
        day >= self.start() && day <= self.end()
    }
}

impl fmt::Display for FlexDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlexDate::Day(d) => write!(f, "{}", d.format("%Y-%m-%d")),
            FlexDate::Week(year, week) => write!(f, "{year}-W{week:02}"),
            FlexDate::Month(year, month) => write!(f, "{year}-{month:02}"),
        }
    }
}

impl FromStr for FlexDate {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
            let d = NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|e| format!("invalid day date: {e}"))?;
            Ok(FlexDate::Day(d))
        } else if s.contains("-W") {
            let parts: Vec<&str> = s.split("-W").collect();
            if parts.len() != 2 {
                return Err(format!("invalid week date: {s}"));
            }
            let year: u16 = parts[0].parse().map_err(|e| format!("invalid year: {e}"))?;
            let week: u8 = parts[1].parse().map_err(|e| format!("invalid week: {e}"))?;
            Ok(FlexDate::Week(year, week))
        } else if s.len() == 7 && s.as_bytes()[4] == b'-' {
            let parts: Vec<&str> = s.split('-').collect();
            let year: u16 = parts[0].parse().map_err(|e| format!("invalid year: {e}"))?;
            let month: u8 = parts[1].parse().map_err(|e| format!("invalid month: {e}"))?;
            Ok(FlexDate::Month(year, month))
        } else {
            Err(format!("unrecognized date format: {s}"))
        }
    }
}

impl Serialize for FlexDate {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for FlexDate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        FlexDate::from_str(&s).map_err(serde::de::Error::custom)
    }
}

// =====================================================================
// Rewrite domain types — new SQLite-based API
// These live in shared::types (not shared:: root) to avoid conflict
// with the Cloudflare Workers types in shared/src/lib.rs
// =====================================================================

/// Domain type for a container (folder or project).
/// status = None → folder; status = Some("active"|"done"|"paused") → project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    /// None = folder; Some("active"|"done"|"paused"|...) = project
    pub status: Option<String>,
    pub parent_container_id: Option<String>,
    pub position: i32,
    pub pinned: bool,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateContainerRequest {
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    /// None = folder. Some("active") = project with status.
    pub status: Option<String>,
    pub parent_container_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateContainerRequest {
    /// None = no change
    pub name: Option<String>,
    /// None = no change; Some(None) = clear; Some(Some(v)) = set
    pub icon: Option<Option<String>>,
    pub description: Option<Option<String>>,
    pub status: Option<Option<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MoveContainerRequest {
    /// None = move to root; Some(id) = move under that parent
    pub parent_container_id: Option<String>,
    /// None = append to end (server computes next_position)
    pub position: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContainerProgress {
    pub total_lists: i64,
    pub total_items: i64,
    pub completed_items: i64,
}

/// Home page data (container-only in B1; B2 adds list fields).
#[derive(Debug, Clone, Serialize)]
pub struct HomeData {
    pub pinned_containers: Vec<Container>,
    pub recent_containers: Vec<Container>,
    pub root_containers: Vec<Container>,
}

// --- sqlx integration (enabled via "sqlx" feature) ---

#[cfg(feature = "sqlx")]
mod sqlx_impl {
    use super::FlexDate;
    use sqlx::encode::IsNull;
    use sqlx::error::BoxDynError;
    use sqlx::sqlite::{SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef};
    use sqlx::{Decode, Encode, Sqlite, Type};
    use std::str::FromStr;

    impl Type<Sqlite> for FlexDate {
        fn type_info() -> SqliteTypeInfo {
            <String as Type<Sqlite>>::type_info()
        }

        fn compatible(ty: &SqliteTypeInfo) -> bool {
            <String as Type<Sqlite>>::compatible(ty)
        }
    }

    impl Encode<'_, Sqlite> for FlexDate {
        fn encode_by_ref(
            &self,
            args: &mut Vec<SqliteArgumentValue<'_>>,
        ) -> Result<IsNull, BoxDynError> {
            let s = self.to_string();
            args.push(SqliteArgumentValue::Text(s.into()));
            Ok(IsNull::No)
        }
    }

    impl Decode<'_, Sqlite> for FlexDate {
        fn decode(value: SqliteValueRef<'_>) -> Result<Self, BoxDynError> {
            let s = <String as Decode<Sqlite>>::decode(value)?;
            FlexDate::from_str(&s).map_err(|e| e.into())
        }
    }
}
