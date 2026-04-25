use crate::DomainError;

/// Tag types that allow at most one instance per entity (item/list/container).
const EXCLUSIVE_TYPES: &[&str] = &["priority"];

/// Reject merging a tag into itself.
pub fn validate_merge(source_id: &str, target_id: &str) -> Result<(), DomainError> {
    if source_id == target_id {
        return Err(DomainError::Validation("merge_same_tag"));
    }
    Ok(())
}

/// Reject setting a tag's parent to itself or to one of its own descendants.
///
/// `ancestor_ids` = IDs returned by `db::tags::get_ancestors(candidate_parent_id)`.
/// If `tag_id` appears in that list, the assignment would create a cycle.
pub fn validate_parent(
    tag_id: &str,
    candidate_parent_id: &str,
    ancestor_ids: &[String],
) -> Result<(), DomainError> {
    if tag_id == candidate_parent_id {
        return Err(DomainError::Validation("tag_cycle_detected"));
    }
    if ancestor_ids.iter().any(|a| a == tag_id) {
        return Err(DomainError::Validation("tag_cycle_detected"));
    }
    Ok(())
}

/// Reject assigning a second exclusive-type tag (e.g. "priority") to the same entity.
///
/// `existing` = Some(existing_tag_id) if a tag of the same type is already assigned, None otherwise.
pub fn validate_exclusive_type(tag_type: &str, existing: Option<&str>) -> Result<(), DomainError> {
    if EXCLUSIVE_TYPES.contains(&tag_type) && existing.is_some() {
        return Err(DomainError::Validation("exclusive_type_conflict"));
    }
    Ok(())
}

/// Enforce location tag hierarchy: city tags need a country parent, address tags need a city parent.
///
/// `parent_tag_type` = None if the tag has no parent.
pub fn validate_location_hierarchy(
    tag_type: &str,
    parent_tag_type: Option<&str>,
) -> Result<(), DomainError> {
    match tag_type {
        "city" if parent_tag_type != Some("country") => {
            return Err(DomainError::Validation("city_requires_country_parent"));
        }
        "address" if parent_tag_type != Some("city") => {
            return Err(DomainError::Validation("address_requires_city_parent"));
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    // validate_merge
    #[test]
    fn merge_same_id_rejected() {
        assert!(validate_merge("t1", "t1").is_err());
    }

    #[test]
    fn merge_different_ids_ok() {
        assert!(validate_merge("t1", "t2").is_ok());
    }

    // validate_parent
    #[test]
    fn parent_self_rejected() {
        assert!(validate_parent("t1", "t1", &[]).is_err());
    }

    #[test]
    fn parent_ancestor_rejected() {
        let ancestors = ids(&["t2", "t1"]);
        assert!(validate_parent("t1", "t3", &ancestors).is_err());
    }

    #[test]
    fn parent_non_ancestor_ok() {
        let ancestors = ids(&["t2", "t3"]);
        assert!(validate_parent("t1", "t4", &ancestors).is_ok());
    }

    #[test]
    fn parent_empty_ancestors_ok() {
        assert!(validate_parent("t1", "t2", &[]).is_ok());
    }

    // validate_exclusive_type
    #[test]
    fn non_exclusive_type_allows_duplicate() {
        assert!(validate_exclusive_type("tag", Some("existing")).is_ok());
    }

    #[test]
    fn priority_without_existing_ok() {
        assert!(validate_exclusive_type("priority", None).is_ok());
    }

    #[test]
    fn priority_with_existing_rejected() {
        assert!(validate_exclusive_type("priority", Some("t1")).is_err());
    }

    // validate_location_hierarchy
    #[test]
    fn generic_tag_no_restriction() {
        assert!(validate_location_hierarchy("tag", None).is_ok());
        assert!(validate_location_hierarchy("tag", Some("anything")).is_ok());
    }

    #[test]
    fn country_tag_no_restriction() {
        assert!(validate_location_hierarchy("country", None).is_ok());
    }

    #[test]
    fn city_requires_country_parent() {
        assert!(validate_location_hierarchy("city", Some("country")).is_ok());
        assert!(validate_location_hierarchy("city", Some("tag")).is_err());
        assert!(validate_location_hierarchy("city", None).is_err());
    }

    #[test]
    fn address_requires_city_parent() {
        assert!(validate_location_hierarchy("address", Some("city")).is_ok());
        assert!(validate_location_hierarchy("address", Some("country")).is_err());
        assert!(validate_location_hierarchy("address", None).is_err());
    }
}
