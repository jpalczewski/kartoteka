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
