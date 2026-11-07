// List validation rules
use crate::DomainError;

/// Validate that a list is not being made its own parent.
/// Deep cycle detection (descendant check) is done in orchestration.
pub fn validate_move(list_id: &str, new_parent_list_id: Option<&str>) -> Result<(), DomainError> {
    if new_parent_list_id == Some(list_id) {
        return Err(DomainError::Validation("cannot_move_to_self"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_to_different_parent_is_valid() {
        assert!(validate_move("list-1", Some("list-2")).is_ok());
    }

    #[test]
    fn move_to_self_is_invalid() {
        let result = validate_move("list-1", Some("list-1"));
        assert!(matches!(
            result,
            Err(DomainError::Validation("cannot_move_to_self"))
        ));
    }

    #[test]
    fn move_to_root_is_valid() {
        assert!(validate_move("list-1", None).is_ok());
    }
}
