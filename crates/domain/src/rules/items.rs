use crate::DomainError;
use kartoteka_shared::types::FlexDate;

pub fn validate_title(title: &str) -> Result<(), DomainError> {
    if title.trim().is_empty() {
        return Err(DomainError::Validation("title_empty"));
    }
    Ok(())
}

pub fn validate_item_dates(
    start_date: Option<&str>,
    start_time: Option<&str>,
    deadline: Option<&str>,
    deadline_time: Option<&str>,
    hard_deadline: Option<&str>,
) -> Result<(), DomainError> {
    if start_time.is_some() && start_date.is_none() {
        return Err(DomainError::Validation("start_time_without_date"));
    }
    if deadline_time.is_some() && deadline.is_none() {
        return Err(DomainError::Validation("deadline_time_without_date"));
    }

    let parse = |s: &str| {
        s.parse::<FlexDate>()
            .map_err(|_| DomainError::Validation("invalid_date"))
    };

    let start: Option<FlexDate> = start_date.map(parse).transpose()?;
    let dl: Option<FlexDate> = deadline.map(parse).transpose()?;
    let hard: Option<FlexDate> = hard_deadline.map(parse).transpose()?;

    if let (Some(s), Some(d)) = (&start, &dl) {
        if s.start() > d.start() {
            return Err(DomainError::Validation("start_date_after_deadline"));
        }
    }
    if let (Some(d), Some(h)) = (&dl, &hard) {
        if d.start() > h.start() {
            return Err(DomainError::Validation("deadline_after_hard_deadline"));
        }
    }
    if let (Some(s), None, Some(h)) = (&start, &dl, &hard) {
        if s.start() > h.start() {
            return Err(DomainError::Validation("start_date_after_hard_deadline"));
        }
    }

    Ok(())
}

/// Returns Err if item uses date/quantity fields but the list lacks the required feature.
pub fn validate_features(
    features: &[String],
    has_date_fields: bool,
    has_quantity_fields: bool,
) -> Result<(), DomainError> {
    if has_date_fields && !features.iter().any(|f| f == "deadlines") {
        return Err(DomainError::FeatureRequired("deadlines"));
    }
    if has_quantity_fields && !features.iter().any(|f| f == "quantity") {
        return Err(DomainError::FeatureRequired("quantity"));
    }
    Ok(())
}

/// Returns true if actual_quantity >= target_quantity (item should be auto-completed).
pub fn should_auto_complete(actual_quantity: i32, target_quantity: i32) -> bool {
    actual_quantity >= target_quantity
}

pub fn validate_can_complete(unresolved_blocker_count: usize) -> Result<(), DomainError> {
    if unresolved_blocker_count > 0 {
        return Err(DomainError::Validation("has_unresolved_blockers"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn features(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_special_fields_always_ok() {
        assert!(validate_features(&[], false, false).is_ok());
    }

    #[test]
    fn dates_require_deadlines_feature() {
        let err = validate_features(&[], true, false).unwrap_err();
        assert!(matches!(err, DomainError::FeatureRequired("deadlines")));
    }

    #[test]
    fn dates_ok_with_deadlines_feature() {
        assert!(validate_features(&features(&["deadlines"]), true, false).is_ok());
    }

    #[test]
    fn quantity_fields_require_quantity_feature() {
        let err = validate_features(&[], false, true).unwrap_err();
        assert!(matches!(err, DomainError::FeatureRequired("quantity")));
    }

    #[test]
    fn quantity_ok_with_quantity_feature() {
        assert!(validate_features(&features(&["quantity"]), false, true).is_ok());
    }

    #[test]
    fn both_features_both_fields_ok() {
        assert!(validate_features(&features(&["deadlines", "quantity"]), true, true).is_ok());
    }

    #[test]
    fn auto_complete_at_target() {
        assert!(should_auto_complete(5, 5));
    }

    #[test]
    fn auto_complete_above_target() {
        assert!(should_auto_complete(6, 5));
    }

    #[test]
    fn no_auto_complete_below_target() {
        assert!(!should_auto_complete(4, 5));
        assert!(!should_auto_complete(0, 5));
    }

    #[test]
    fn validate_can_complete_rejects_when_blockers_exist() {
        let err = validate_can_complete(2).unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation("has_unresolved_blockers")
        ));
    }

    #[test]
    fn validate_can_complete_passes_when_no_blockers() {
        assert!(validate_can_complete(0).is_ok());
    }

    #[test]
    fn title_empty_rejected() {
        assert!(matches!(
            validate_title(""),
            Err(DomainError::Validation("title_empty"))
        ));
        assert!(matches!(
            validate_title("   "),
            Err(DomainError::Validation("title_empty"))
        ));
    }

    #[test]
    fn title_nonempty_ok() {
        assert!(validate_title("Buy milk").is_ok());
        assert!(validate_title(" x ").is_ok());
    }

    #[test]
    fn dates_all_none_ok() {
        assert!(validate_item_dates(None, None, None, None, None).is_ok());
    }

    #[test]
    fn dates_valid_order_ok() {
        assert!(
            validate_item_dates(
                Some("2026-05-01"),
                None,
                Some("2026-05-10"),
                None,
                Some("2026-05-20"),
            )
            .is_ok()
        );
    }

    #[test]
    fn start_date_after_deadline_rejected() {
        let err = validate_item_dates(Some("2026-05-10"), None, Some("2026-05-01"), None, None)
            .unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation("start_date_after_deadline")
        ));
    }

    #[test]
    fn deadline_after_hard_deadline_rejected() {
        let err = validate_item_dates(None, None, Some("2026-05-30"), None, Some("2026-05-20"))
            .unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation("deadline_after_hard_deadline")
        ));
    }

    #[test]
    fn start_time_without_date_rejected() {
        let err = validate_item_dates(None, Some("09:00"), None, None, None).unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation("start_time_without_date")
        ));
    }

    #[test]
    fn deadline_time_without_date_rejected() {
        let err = validate_item_dates(None, None, None, Some("18:00"), None).unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation("deadline_time_without_date")
        ));
    }

    #[test]
    fn start_after_hard_deadline_without_deadline_rejected() {
        let err = validate_item_dates(Some("2026-05-25"), None, None, None, Some("2026-05-20"))
            .unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation("start_date_after_hard_deadline")
        ));
    }

    #[test]
    fn week_and_month_formats_accepted() {
        // Week deadline after day start — both formats must parse without error
        assert!(
            validate_item_dates(Some("2026-05-01"), None, Some("2026-W20"), None, None).is_ok()
        );
        // Month hard_deadline after week deadline
        assert!(validate_item_dates(None, None, Some("2026-W20"), None, Some("2026-06")).is_ok());
    }

    #[test]
    fn week_format_order_violation_rejected() {
        // W18 starts 2026-04-27, W17 starts 2026-04-20 — so start > deadline
        let err =
            validate_item_dates(Some("2026-W18"), None, Some("2026-W17"), None, None).unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation("start_date_after_deadline")
        ));
    }
}
