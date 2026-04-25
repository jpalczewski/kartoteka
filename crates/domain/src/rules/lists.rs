use crate::DomainError;

/// Validate that the given features are compatible with the list_type.
///
/// Rules:
/// - `shopping`  → requires "quantity" feature (items need qty/unit fields)
/// - `habits`    → requires "deadlines" feature (items need date scheduling)
/// - `checklist` → any features allowed
/// - `log`       → any features allowed
/// - unknown     → rejected
pub fn validate_list_type_features(
    list_type: &str,
    features: &[String],
) -> Result<(), DomainError> {
    match list_type {
        "shopping" => {
            if !features.iter().any(|f| f == "quantity") {
                return Err(DomainError::Validation("shopping_lists_require_quantity"));
            }
        }
        "habits" => {
            if !features.iter().any(|f| f == "deadlines") {
                return Err(DomainError::Validation("habits_lists_require_deadlines"));
            }
        }
        "checklist" | "log" => {}
        _ => return Err(DomainError::Validation("unknown_list_type")),
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
    fn checklist_allows_no_features() {
        assert!(validate_list_type_features("checklist", &[]).is_ok());
    }

    #[test]
    fn checklist_allows_any_features() {
        assert!(
            validate_list_type_features("checklist", &features(&["deadlines", "quantity"])).is_ok()
        );
    }

    #[test]
    fn log_allows_any_features() {
        assert!(validate_list_type_features("log", &features(&["deadlines"])).is_ok());
    }

    #[test]
    fn shopping_requires_quantity() {
        let err = validate_list_type_features("shopping", &[]).unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation("shopping_lists_require_quantity")
        ));
    }

    #[test]
    fn shopping_ok_with_quantity() {
        assert!(validate_list_type_features("shopping", &features(&["quantity"])).is_ok());
    }

    #[test]
    fn shopping_ok_with_quantity_and_more() {
        assert!(
            validate_list_type_features("shopping", &features(&["quantity", "deadlines"])).is_ok()
        );
    }

    #[test]
    fn habits_requires_deadlines() {
        let err = validate_list_type_features("habits", &[]).unwrap_err();
        assert!(matches!(
            err,
            DomainError::Validation("habits_lists_require_deadlines")
        ));
    }

    #[test]
    fn habits_ok_with_deadlines() {
        assert!(validate_list_type_features("habits", &features(&["deadlines"])).is_ok());
    }

    #[test]
    fn unknown_type_rejects() {
        let err = validate_list_type_features("kanban", &[]).unwrap_err();
        assert!(matches!(err, DomainError::Validation("unknown_list_type")));
    }
}
