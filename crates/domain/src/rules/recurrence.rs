//! Recurrence rules: date stepping, validation and next-occurrence dates.
//!
//! Pure logic only — no DB. The rule format is `"<interval><unit>"` (`7d`, `2w`, `1m`)
//! and the [`RecurrenceMode`] decides whether the next occurrence is dated from the
//! scheduled date (`fixed`) or from the completion date (`floating`).

use crate::DomainError;
use chrono::{Datelike, Days, Months, NaiveDate};
use kartoteka_shared::types::{FlexDate, RecurrenceMode, RecurrenceRule, RecurrenceUnit};

/// The three date fields of an item, as needed by recurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemDates {
    pub start_date: Option<FlexDate>,
    pub deadline: Option<FlexDate>,
    pub hard_deadline: Option<FlexDate>,
}

/// Validate a recurrence rule/mode pair against the item's dates.
///
/// - a mode without a rule is rejected (`recurrence_mode_without_rule`),
/// - a rule needs a `deadline` or a `start_date` (`recurrence_requires_date`),
/// - every set date field must fit the rule unit (`recurrence_incompatible_date`):
///   `Day` accepts `d`/`w`/`m`, `Week` only `w`, `Month` only `m`.
/// # Errors
/// See the keys above.
pub fn validate_recurrence(
    rule: Option<&RecurrenceRule>,
    mode: Option<RecurrenceMode>,
    dates: &ItemDates,
) -> Result<(), DomainError> {
    let Some(rule) = rule else {
        if mode.is_some() {
            return Err(DomainError::Validation("recurrence_mode_without_rule"));
        }
        return Ok(());
    };

    if dates.deadline.is_none() && dates.start_date.is_none() {
        return Err(DomainError::Validation("recurrence_requires_date"));
    }

    let fields = [&dates.start_date, &dates.deadline, &dates.hard_deadline];
    for date in fields.into_iter().flatten() {
        if !is_compatible(date, rule.unit) {
            return Err(DomainError::Validation("recurrence_incompatible_date"));
        }
    }

    Ok(())
}

/// One step of the rule on a single date.
///
/// Precondition: the date is compatible with the rule unit (see
/// [`validate_recurrence`]); an incompatible pair is returned unchanged.
///
/// Day + `Nm` keeps a month-end date anchored to the month end
/// (31.01 → 28.02 → 31.03); every other day clamps (`checked_add_months`), so
/// 30.01 + 1m → 28.02 and then 28.02 + 1m → 31.03. The day-30 → month-end drift
/// is accepted: `step` is stateless.
pub fn step(date: &FlexDate, rule: &RecurrenceRule) -> FlexDate {
    let interval = u32::from(rule.interval);
    match (date, rule.unit) {
        (FlexDate::Day(day), RecurrenceUnit::Day) => FlexDate::Day(add_days(*day, interval)),
        (FlexDate::Day(day), RecurrenceUnit::Week) => {
            FlexDate::Day(add_days(*day, interval.saturating_mul(7)))
        }
        (FlexDate::Day(day), RecurrenceUnit::Month) => FlexDate::Day(add_months(*day, interval)),
        (FlexDate::Week(..), RecurrenceUnit::Week) => {
            week_of(add_days(date.start(), interval.saturating_mul(7)))
        }
        (FlexDate::Month(year, month), RecurrenceUnit::Month) => {
            step_month(*year, *month, interval)
        }
        // Incompatible combinations are a precondition violation.
        _ => date.clone(),
    }
}

/// Dates of the next occurrence according to the mode.
///
/// **Fixed:** every set field is stepped independently; `today` is ignored.
///
/// **Floating:** the anchor is `deadline ?? start_date`; the base is `today`
/// truncated to the anchor's granularity, the new anchor is `step(base)` and the
/// other fields move by the day delta between the two anchors (`Week`/`Month`
/// fields become the period that contains their shifted start).
pub fn next_dates(
    dates: &ItemDates,
    rule: &RecurrenceRule,
    mode: RecurrenceMode,
    today: NaiveDate,
) -> ItemDates {
    if mode == RecurrenceMode::Fixed {
        return ItemDates {
            start_date: dates.start_date.as_ref().map(|d| step(d, rule)),
            deadline: dates.deadline.as_ref().map(|d| step(d, rule)),
            hard_deadline: dates.hard_deadline.as_ref().map(|d| step(d, rule)),
        };
    }

    let Some(anchor) = dates.deadline.as_ref().or(dates.start_date.as_ref()) else {
        return dates.clone();
    };

    let new_anchor = step(&truncate(today, anchor), rule);
    let delta = new_anchor.start().signed_duration_since(anchor.start());

    let mut next = ItemDates {
        start_date: dates.start_date.as_ref().map(|d| shift(d, delta)),
        deadline: dates.deadline.as_ref().map(|d| shift(d, delta)),
        hard_deadline: dates.hard_deadline.as_ref().map(|d| shift(d, delta)),
    };

    // The anchor itself follows `step`, not the plain shift.
    if dates.deadline.is_some() {
        next.deadline = Some(new_anchor);
    } else {
        next.start_date = Some(new_anchor);
    }

    next
}

/// Parse a rule string (`"7d"`, `"2w"`, `"1m"`, interval 1–999).
/// # Errors
/// `Validation("recurrence_invalid_rule")` for anything else.
pub fn parse_rule(s: &str) -> Result<RecurrenceRule, DomainError> {
    s.parse::<RecurrenceRule>()
        .map_err(|_| DomainError::Validation("recurrence_invalid_rule"))
}

/// Parse a mode string (`"fixed"` | `"floating"`).
/// # Errors
/// `Validation("recurrence_invalid_mode")` for anything else.
pub fn parse_mode(s: &str) -> Result<RecurrenceMode, DomainError> {
    s.parse::<RecurrenceMode>()
        .map_err(|_| DomainError::Validation("recurrence_invalid_mode"))
}

fn is_compatible(date: &FlexDate, unit: RecurrenceUnit) -> bool {
    matches!(
        (date, unit),
        (FlexDate::Day(_), _)
            | (FlexDate::Week(..), RecurrenceUnit::Week)
            | (FlexDate::Month(..), RecurrenceUnit::Month)
    )
}

fn add_days(day: NaiveDate, days: u32) -> NaiveDate {
    day.checked_add_days(Days::new(u64::from(days)))
        .unwrap_or(day)
}

/// Add calendar months, keeping a month-end date on the month end.
fn add_months(day: NaiveDate, months: u32) -> NaiveDate {
    let Some(clamped) = day.checked_add_months(Months::new(months)) else {
        return day;
    };
    if day.day() == last_day_of_month(day.year(), day.month()).day() {
        last_day_of_month(clamped.year(), clamped.month())
    } else {
        clamped
    }
}

fn step_month(year: u16, month: u8, months: u32) -> FlexDate {
    let index = i32::from(year) * 12 + i32::from(month) - 1 + i32::try_from(months).unwrap_or(0);
    FlexDate::Month(
        u16::try_from(index.div_euclid(12)).unwrap_or(year),
        u8::try_from(index.rem_euclid(12) + 1).unwrap_or(month),
    )
}

fn last_day_of_month(year: i32, month: u32) -> NaiveDate {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|first| first.pred_opt())
        .expect("valid month")
}

/// The [`FlexDate`] of `day` at `anchor`'s granularity.
fn truncate(day: NaiveDate, anchor: &FlexDate) -> FlexDate {
    match anchor {
        FlexDate::Day(_) => FlexDate::Day(day),
        FlexDate::Week(..) => week_of(day),
        FlexDate::Month(..) => month_of(day),
    }
}

/// Move `date` by `delta` days, keeping its granularity (period containment).
fn shift(date: &FlexDate, delta: chrono::TimeDelta) -> FlexDate {
    let moved = date
        .start()
        .checked_add_signed(delta)
        .unwrap_or_else(|| date.start());
    match date {
        FlexDate::Day(_) => FlexDate::Day(moved),
        FlexDate::Week(..) => week_of(moved),
        FlexDate::Month(..) => month_of(moved),
    }
}

fn week_of(day: NaiveDate) -> FlexDate {
    let iso = day.iso_week();
    FlexDate::Week(
        u16::try_from(iso.year()).unwrap_or(u16::MAX),
        u8::try_from(iso.week()).unwrap_or(1),
    )
}

fn month_of(day: NaiveDate) -> FlexDate {
    FlexDate::Month(
        u16::try_from(day.year()).unwrap_or(u16::MAX),
        u8::try_from(day.month()).unwrap_or(1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(interval: u16, unit: RecurrenceUnit) -> RecurrenceRule {
        RecurrenceRule { interval, unit }
    }

    fn day(y: i32, m: u32, d: u32) -> FlexDate {
        FlexDate::Day(NaiveDate::from_ymd_opt(y, m, d).unwrap())
    }

    fn dates(
        start_date: Option<FlexDate>,
        deadline: Option<FlexDate>,
        hard_deadline: Option<FlexDate>,
    ) -> ItemDates {
        ItemDates {
            start_date,
            deadline,
            hard_deadline,
        }
    }

    // ── step: Day + months ───────────────────────────────────────────────────

    #[test]
    fn step_day_month_from_last_day_anchors() {
        let monthly = rule(1, RecurrenceUnit::Month);
        let feb = step(&day(2026, 1, 31), &monthly);
        assert_eq!(feb, day(2026, 2, 28));
        assert_eq!(step(&feb, &monthly), day(2026, 3, 31));
    }

    #[test]
    fn step_day_month_plain_clamps() {
        assert_eq!(
            step(&day(2026, 1, 30), &rule(1, RecurrenceUnit::Month)),
            day(2026, 2, 28)
        );
    }

    #[test]
    fn step_day_month_reanchors_after_clamp() {
        assert_eq!(
            step(&day(2026, 2, 28), &rule(1, RecurrenceUnit::Month)),
            day(2026, 3, 31)
        );
    }

    #[test]
    fn step_day_month_mid_month_keeps_day() {
        let monthly = rule(1, RecurrenceUnit::Month);
        let feb = step(&day(2026, 1, 15), &monthly);
        assert_eq!(feb, day(2026, 2, 15));
        assert_eq!(step(&feb, &monthly), day(2026, 3, 15));
    }

    #[test]
    fn step_day_leap_day_plus_twelve_months() {
        assert_eq!(
            step(&day(2028, 2, 29), &rule(12, RecurrenceUnit::Month)),
            day(2029, 2, 28)
        );
    }

    // ── step: Day + days / weeks, Week, Month ────────────────────────────────

    #[test]
    fn step_day_days_and_weeks() {
        assert_eq!(
            step(&day(2026, 5, 1), &rule(7, RecurrenceUnit::Day)),
            day(2026, 5, 8)
        );
        assert_eq!(
            step(&day(2026, 5, 1), &rule(2, RecurrenceUnit::Week)),
            day(2026, 5, 15)
        );
    }

    #[test]
    fn step_week_rolls_into_next_iso_year() {
        let weekly = rule(1, RecurrenceUnit::Week);
        let w53 = step(&FlexDate::Week(2026, 52), &weekly);
        assert_eq!(w53, FlexDate::Week(2026, 53));
        assert_eq!(step(&w53, &weekly), FlexDate::Week(2027, 1));
    }

    #[test]
    fn step_month_rolls_over_year() {
        assert_eq!(
            step(&FlexDate::Month(2026, 12), &rule(1, RecurrenceUnit::Month)),
            FlexDate::Month(2027, 1)
        );
        assert_eq!(
            step(&FlexDate::Month(2026, 11), &rule(3, RecurrenceUnit::Month)),
            FlexDate::Month(2027, 2)
        );
    }

    #[test]
    fn step_incompatible_pair_is_unchanged() {
        assert_eq!(
            step(&FlexDate::Week(2026, 5), &rule(1, RecurrenceUnit::Month)),
            FlexDate::Week(2026, 5)
        );
        assert_eq!(
            step(&FlexDate::Month(2026, 5), &rule(1, RecurrenceUnit::Day)),
            FlexDate::Month(2026, 5)
        );
    }

    // ── validate_recurrence ──────────────────────────────────────────────────

    #[test]
    fn validate_no_rule_no_mode_ok() {
        assert!(validate_recurrence(None, None, &dates(None, None, None)).is_ok());
        assert!(validate_recurrence(None, None, &dates(Some(day(2026, 1, 1)), None, None)).is_ok());
    }

    #[test]
    fn validate_mode_without_rule_rejected() {
        assert!(matches!(
            validate_recurrence(
                None,
                Some(RecurrenceMode::Floating),
                &dates(None, None, None)
            ),
            Err(DomainError::Validation("recurrence_mode_without_rule"))
        ));
    }

    #[test]
    fn validate_rule_without_dates_rejected() {
        let weekly = rule(1, RecurrenceUnit::Week);
        assert!(matches!(
            validate_recurrence(Some(&weekly), None, &dates(None, None, None)),
            Err(DomainError::Validation("recurrence_requires_date"))
        ));
        // A hard deadline alone does not anchor a rule.
        assert!(matches!(
            validate_recurrence(
                Some(&weekly),
                Some(RecurrenceMode::Fixed),
                &dates(None, None, Some(FlexDate::Week(2026, 5)))
            ),
            Err(DomainError::Validation("recurrence_requires_date"))
        ));
    }

    #[test]
    fn validate_rule_with_mode_and_date_ok() {
        let weekly = rule(2, RecurrenceUnit::Week);
        assert!(
            validate_recurrence(
                Some(&weekly),
                Some(RecurrenceMode::Floating),
                &dates(Some(FlexDate::Week(2026, 5)), None, None)
            )
            .is_ok()
        );
    }

    #[test]
    fn validate_day_accepts_all_units() {
        let d = dates(Some(day(2026, 1, 1)), None, None);
        for unit in [
            RecurrenceUnit::Day,
            RecurrenceUnit::Week,
            RecurrenceUnit::Month,
        ] {
            assert!(validate_recurrence(Some(&rule(1, unit)), None, &d).is_ok());
        }
    }

    #[test]
    fn validate_week_accepts_week_only() {
        let d = dates(Some(FlexDate::Week(2026, 5)), None, None);
        assert!(validate_recurrence(Some(&rule(1, RecurrenceUnit::Week)), None, &d).is_ok());
        for unit in [RecurrenceUnit::Day, RecurrenceUnit::Month] {
            assert!(matches!(
                validate_recurrence(Some(&rule(1, unit)), None, &d),
                Err(DomainError::Validation("recurrence_incompatible_date"))
            ));
        }
    }

    #[test]
    fn validate_month_accepts_month_only() {
        let d = dates(Some(FlexDate::Month(2026, 5)), None, None);
        assert!(validate_recurrence(Some(&rule(1, RecurrenceUnit::Month)), None, &d).is_ok());
        for unit in [RecurrenceUnit::Day, RecurrenceUnit::Week] {
            assert!(matches!(
                validate_recurrence(Some(&rule(1, unit)), None, &d),
                Err(DomainError::Validation("recurrence_incompatible_date"))
            ));
        }
    }

    #[test]
    fn validate_every_set_field_must_be_compatible() {
        let weekly = rule(1, RecurrenceUnit::Week);
        assert!(matches!(
            validate_recurrence(
                Some(&weekly),
                None,
                &dates(
                    Some(FlexDate::Week(2026, 5)),
                    Some(FlexDate::Week(2026, 6)),
                    Some(FlexDate::Month(2026, 7))
                )
            ),
            Err(DomainError::Validation("recurrence_incompatible_date"))
        ));
        let monthly = rule(1, RecurrenceUnit::Month);
        assert!(
            validate_recurrence(
                Some(&monthly),
                None,
                &dates(
                    Some(FlexDate::Month(2026, 5)),
                    Some(FlexDate::Month(2026, 6)),
                    Some(FlexDate::Month(2026, 7))
                )
            )
            .is_ok()
        );
    }

    // ── next_dates: fixed ────────────────────────────────────────────────────

    #[test]
    fn fixed_steps_each_field_independently() {
        let next = next_dates(
            &dates(
                Some(day(2026, 1, 31)),
                Some(day(2026, 2, 10)),
                Some(FlexDate::Month(2026, 4)),
            ),
            &rule(1, RecurrenceUnit::Month),
            RecurrenceMode::Fixed,
            NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
        );
        assert_eq!(next.start_date, Some(day(2026, 2, 28)));
        assert_eq!(next.deadline, Some(day(2026, 3, 10)));
        assert_eq!(next.hard_deadline, Some(FlexDate::Month(2026, 5)));
    }

    #[test]
    fn fixed_ignores_today() {
        let d = dates(Some(day(2026, 1, 15)), None, None);
        let monthly = rule(1, RecurrenceUnit::Month);
        let early = next_dates(
            &d,
            &monthly,
            RecurrenceMode::Fixed,
            NaiveDate::from_ymd_opt(2020, 3, 1).unwrap(),
        );
        let late = next_dates(
            &d,
            &monthly,
            RecurrenceMode::Fixed,
            NaiveDate::from_ymd_opt(2030, 7, 20).unwrap(),
        );
        assert_eq!(early, late);
        assert_eq!(early.start_date, Some(day(2026, 2, 15)));
    }

    // ── next_dates: floating ─────────────────────────────────────────────────

    #[test]
    fn floating_keeps_offset_between_fields() {
        let next = next_dates(
            &dates(Some(day(2026, 1, 7)), Some(day(2026, 1, 10)), None),
            &rule(3, RecurrenceUnit::Day),
            RecurrenceMode::Floating,
            NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
        );
        assert_eq!(next.deadline, Some(day(2026, 2, 4)));
        assert_eq!(next.start_date, Some(day(2026, 2, 1)));
    }

    #[test]
    fn floating_overdue_anchor_steps_from_today() {
        let next = next_dates(
            &dates(None, Some(day(2025, 1, 1)), None),
            &rule(3, RecurrenceUnit::Day),
            RecurrenceMode::Floating,
            NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
        );
        assert_eq!(next.deadline, Some(day(2026, 2, 4)));
        assert_eq!(next.start_date, None);
    }

    #[test]
    fn floating_month_anchor_uses_month_of_today() {
        let next = next_dates(
            &dates(Some(day(2026, 1, 10)), Some(FlexDate::Month(2026, 1)), None),
            &rule(1, RecurrenceUnit::Month),
            RecurrenceMode::Floating,
            NaiveDate::from_ymd_opt(2026, 3, 15).unwrap(),
        );
        assert_eq!(next.deadline, Some(FlexDate::Month(2026, 4)));
        // delta = 2026-04-01 - 2026-01-01 = 90 days → 2026-01-10 + 90 = 2026-04-10
        assert_eq!(next.start_date, Some(day(2026, 4, 10)));
    }

    #[test]
    fn floating_uses_start_date_when_no_deadline() {
        let next = next_dates(
            &dates(Some(FlexDate::Week(2026, 5)), None, None),
            &rule(1, RecurrenceUnit::Week),
            RecurrenceMode::Floating,
            NaiveDate::from_ymd_opt(2026, 3, 4).unwrap(),
        );
        // 2026-03-04 is in ISO week 2026-W10 → new anchor 2026-W11.
        assert_eq!(next.start_date, Some(FlexDate::Week(2026, 11)));
    }

    #[test]
    fn floating_other_fields_become_period_containing_shifted_start() {
        let next = next_dates(
            &dates(Some(FlexDate::Week(2026, 5)), Some(day(2026, 2, 2)), None),
            &rule(1, RecurrenceUnit::Month),
            RecurrenceMode::Floating,
            NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(),
        );
        // 2026-W05 starts 2026-01-26; delta = 2026-04-02 - 2026-02-02 = 59 days
        // → 2026-03-26, which is in ISO week 2026-W13.
        assert_eq!(next.deadline, Some(day(2026, 4, 2)));
        assert_eq!(next.start_date, Some(FlexDate::Week(2026, 13)));
    }

    // ── parse_rule / parse_mode ──────────────────────────────────────────────

    #[test]
    fn parse_rule_round_trips() {
        assert_eq!(parse_rule("7d").unwrap(), rule(7, RecurrenceUnit::Day));
        assert_eq!(parse_rule("2w").unwrap(), rule(2, RecurrenceUnit::Week));
        assert_eq!(parse_rule("1m").unwrap(), rule(1, RecurrenceUnit::Month));
        assert_eq!(parse_rule("999d").unwrap().to_string(), "999d");
    }

    #[test]
    fn parse_rule_rejects_invalid() {
        for s in ["", "0d", "1000d", "1y", "d", "-1d", "1 d", "01d"] {
            assert!(matches!(
                parse_rule(s),
                Err(DomainError::Validation("recurrence_invalid_rule"))
            ));
        }
    }

    #[test]
    fn parse_mode_round_trips() {
        assert_eq!(parse_mode("fixed").unwrap(), RecurrenceMode::Fixed);
        assert_eq!(parse_mode("floating").unwrap(), RecurrenceMode::Floating);
    }

    #[test]
    fn parse_mode_rejects_invalid() {
        for s in ["", "Fixed", "weekly", " fixed"] {
            assert!(matches!(
                parse_mode(s),
                Err(DomainError::Validation("recurrence_invalid_mode"))
            ));
        }
    }
}
