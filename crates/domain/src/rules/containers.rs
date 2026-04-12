use crate::DomainError;

/// Validate that a parent container is a folder (status IS NULL), not a project.
/// Called before creating or moving a container under a parent.
pub fn validate_hierarchy(parent_status: Option<&str>) -> Result<(), DomainError> {
    if parent_status.is_some() {
        return Err(DomainError::Validation("invalid_container_hierarchy"));
    }
    Ok(())
}

/// Validate that a container is not being moved to itself.
/// Deep cycle detection (descendant check) is done in orchestration.
pub fn validate_move(container_id: &str, new_parent_id: Option<&str>) -> Result<(), DomainError> {
    if new_parent_id == Some(container_id) {
        return Err(DomainError::Validation("cannot_move_to_self"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_is_valid_parent() {
        assert!(validate_hierarchy(None).is_ok());
    }

    #[test]
    fn project_is_invalid_parent() {
        let result = validate_hierarchy(Some("active"));
        assert!(matches!(result, Err(DomainError::Validation("invalid_container_hierarchy"))));
    }

    #[test]
    fn any_status_rejects_as_parent() {
        assert!(validate_hierarchy(Some("done")).is_err());
        assert!(validate_hierarchy(Some("paused")).is_err());
        assert!(validate_hierarchy(Some("anything")).is_err());
    }

    #[test]
    fn move_to_different_parent_is_valid() {
        assert!(validate_move("container-1", Some("container-2")).is_ok());
    }

    #[test]
    fn move_to_self_is_invalid() {
        let result = validate_move("container-1", Some("container-1"));
        assert!(matches!(result, Err(DomainError::Validation("cannot_move_to_self"))));
    }

    #[test]
    fn move_to_root_is_valid() {
        assert!(validate_move("container-1", None).is_ok());
    }
}
