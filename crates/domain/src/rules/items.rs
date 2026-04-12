use crate::DomainError;

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

/// Placeholder — full blocker enforcement is in E2. Always returns Ok in B3.
pub fn validate_can_complete(_blockers: &[&str]) -> Result<(), DomainError> {
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
    fn validate_can_complete_always_ok() {
        assert!(validate_can_complete(&[]).is_ok());
        assert!(validate_can_complete(&["blocker"]).is_ok());
    }
}
