use chrono::{Datelike, NaiveDate, NaiveTime};

pub const BUSINESS_DATE_MIN_YEAR: i32 = 2000;
pub const BUSINESS_DATE_MAX_YEAR: i32 = 2100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateValidationError {
    Invalid,
    OutOfRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeValidationError {
    Invalid,
}

pub fn validate_business_date(date_str: &str) -> Result<NaiveDate, DateValidationError> {
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|_| DateValidationError::Invalid)?;
    if !(BUSINESS_DATE_MIN_YEAR..=BUSINESS_DATE_MAX_YEAR).contains(&date.year()) {
        return Err(DateValidationError::OutOfRange);
    }
    Ok(date)
}

pub fn validate_hhmm_time(time_str: &str) -> Result<NaiveTime, TimeValidationError> {
    NaiveTime::parse_from_str(time_str, "%H:%M").map_err(|_| TimeValidationError::Invalid)
}
