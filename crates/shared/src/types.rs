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
            let month: u8 = parts[1]
                .parse()
                .map_err(|e| format!("invalid month: {e}"))?;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContainerRequest {
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    /// None = folder. Some("active") = project with status.
    pub status: Option<String>,
    pub parent_container_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateContainerRequest {
    /// None = no change
    pub name: Option<String>,
    /// None = no change; Some(None) = clear; Some(Some(v)) = set
    pub icon: Option<Option<String>>,
    /// None = no change; Some(None) = clear; Some(Some(v)) = set
    pub description: Option<Option<String>>,
    /// None = no change; Some(None) = clear; Some(Some(v)) = set
    pub status: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeData {
    pub pinned_containers: Vec<Container>,
    pub recent_containers: Vec<Container>,
    pub root_containers: Vec<Container>,
    pub pinned_lists: Vec<List>,
    pub recent_lists: Vec<List>,
    pub root_lists: Vec<List>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListFeature {
    pub feature_name: String,
    pub config: serde_json::Value,
}

/// Shared List type for server function return values.
/// Mirrors domain::lists::List — kept in shared so WASM hydrate build can deserialize.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub list_type: String,
    pub parent_list_id: Option<String>,
    pub position: i64,
    pub archived: bool,
    pub container_id: Option<String>,
    pub pinned: bool,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub features: Vec<ListFeature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub parent_tag_id: Option<String>,
    pub tag_type: String,
    pub metadata: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTagLink {
    pub list_id: String,
    pub tag_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateListRequest {
    pub name: String,
    pub list_type: Option<String>,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub container_id: Option<String>,
    pub parent_list_id: Option<String>,
    /// Feature name strings, e.g. "deadlines", "quantity".
    #[serde(default)]
    pub features: Vec<String>,
}

/// Shared Item type for server function return values.
/// Mirrors domain::items::Item — kept in shared so WASM hydrate build can deserialize.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub description: Option<String>,
    pub completed: bool,
    pub position: i32,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<FlexDate>,
    pub start_time: Option<String>,
    pub deadline: Option<FlexDate>,
    pub deadline_time: Option<String>,
    pub hard_deadline: Option<FlexDate>,
    pub estimated_duration: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

/// Payload returned by `get_list_data` — list header + items + sublists in one call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListData {
    pub list: List,
    pub items: Vec<Item>,
    pub sublists: Vec<List>,
    /// `list.created_at` converted to the requesting user's timezone, formatted for display.
    pub created_at_local: String,
}

/// Item enriched with its parent list's display name.
/// Used in Today and CalendarDay pages where items from multiple lists are shown together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateItem {
    pub item: Item,
    pub list_name: String,
}

/// Items for a Today page: today's items and items with past-due deadlines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodayData {
    /// Resolved today date as "YYYY-MM-DD" (in user's timezone).
    pub today_date: String,
    /// Items with start_date, deadline, or hard_deadline = today.
    pub today: Vec<DateItem>,
    /// Incomplete items with deadline strictly before today.
    pub overdue: Vec<DateItem>,
}

/// One day in a calendar month view with its item count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarDay {
    /// Date as "YYYY-MM-DD".
    pub date: String,
    /// Number of items falling on this date (counting start_date, deadline, hard_deadline each once per item).
    pub count: u32,
}

/// Everything needed to render a calendar month grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarMonthData {
    pub year: i32,
    pub month: u32,
    /// The resolved year_month string, e.g. "2026-04". Returned so the component knows what it got.
    pub year_month: String,
    /// Weekday of the first day of the month: 0 = Monday, 6 = Sunday (ISO weekday - 1).
    pub first_weekday: u8,
    /// Number of days in the month (28–31).
    pub days_in_month: u8,
    /// Per-day item counts — only days that have at least one item are included.
    pub items_by_day: Vec<CalendarDay>,
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

// --- Comments ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub content: String,
    pub author_type: String, // "user" | "assistant"
    pub author_name: Option<String>,
    pub user_id: String,
    pub created_at: String,
    pub updated_at: String,
}

// --- Relations ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    pub from_type: String,
    pub from_id: String,
    pub to_type: String,
    pub to_id: String,
    pub relation_type: String,
    pub user_id: String,
    pub created_at: String,
}

// --- Time Entries ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntry {
    pub id: String,
    pub item_id: Option<String>,
    pub user_id: String,
    pub description: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration: Option<i32>,
    pub source: String,
    pub mode: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemTimeSummary {
    pub total_seconds: i64,
    pub entry_count: i64,
}

// --- Settings ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSetting {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

// --- Tokens ---

/// Metadata for a personal API token (the JWT string is NOT included — returned only at creation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub id: String,
    pub name: String,
    pub scope: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

/// Returned once when a new token is created. Show `token` to the user and discard — it is not stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCreated {
    pub id: String,
    pub token: String,
    pub name: String,
    pub scope: String,
}

// --- Tag detail ---

/// Tag header + lists linked to this tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagDetailData {
    pub tag: Tag,
    pub linked_lists: Vec<List>,
}

// --- Container detail ---

/// Container header + its direct lists + its direct child containers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerData {
    pub container: Container,
    pub lists: Vec<List>,
    pub children: Vec<Container>,
}

// --- Templates ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateItem {
    pub id: String,
    pub template_id: String,
    pub title: String,
    pub description: Option<String>,
    pub position: i32,
    pub quantity: Option<i32>,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateWithItems {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub items: Vec<TemplateItem>,
    pub tag_ids: Vec<String>,
    pub created_at: String,
}
