//! Pure reorder helpers: rearrange lists of ids, rebuild typed vecs.

/// Given a slice of ids and a dragged id, return a new order where the dragged
/// id is moved to just before `before_id` (or to the end if None).
/// Returns None if the order does not change or if any id is missing.
pub fn reorder_ids(
    ids: &[String],
    dragged_id: &str,
    before_id: Option<&str>,
) -> Option<Vec<String>> {
    let source_index = ids.iter().position(|id| id == dragged_id)?;
    let mut next_ids = ids.to_vec();
    let dragged = next_ids.remove(source_index);
    let target_index = match before_id {
        Some(target_id) => next_ids.iter().position(|id| id == target_id)?,
        None => next_ids.len(),
    };
    next_ids.insert(target_index, dragged);
    if next_ids == ids {
        None
    } else {
        Some(next_ids)
    }
}

/// Rebuild `items` so their order matches `ordered_ids`. Returns true if the
/// vec actually changed.
pub fn apply_reorder<T, F>(items: &mut Vec<T>, ordered_ids: &[String], get_id: F) -> bool
where
    T: Clone,
    F: Fn(&T) -> &str,
{
    if items.len() != ordered_ids.len() {
        return false;
    }
    let current_ids: Vec<String> = items.iter().map(|it| get_id(it).to_string()).collect();
    if current_ids == ordered_ids {
        return false;
    }
    let reordered: Option<Vec<T>> = ordered_ids
        .iter()
        .map(|tid| items.iter().find(|it| get_id(it) == tid).cloned())
        .collect();
    match reordered {
        Some(next) => {
            *items = next;
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reorder_ids_moves_before_target() {
        let ids = vec!["a".into(), "b".into(), "c".into()];
        let next = reorder_ids(&ids, "c", Some("b")).unwrap();
        assert_eq!(next, vec!["a", "c", "b"]);
    }

    #[test]
    fn reorder_ids_moves_to_end() {
        let ids = vec!["a".into(), "b".into(), "c".into()];
        let next = reorder_ids(&ids, "a", None).unwrap();
        assert_eq!(next, vec!["b", "c", "a"]);
    }

    #[test]
    fn reorder_ids_no_change_returns_none() {
        let ids = vec!["a".into(), "b".into(), "c".into()];
        assert!(reorder_ids(&ids, "a", Some("b")).is_none());
    }

    #[test]
    fn reorder_ids_missing_id_returns_none() {
        let ids = vec!["a".into(), "b".into()];
        assert!(reorder_ids(&ids, "missing", None).is_none());
    }

    #[test]
    fn apply_reorder_rebuilds_vec() {
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct T {
            id: String,
        }
        let mut items = vec![
            T { id: "a".into() },
            T { id: "b".into() },
            T { id: "c".into() },
        ];
        let changed = apply_reorder(&mut items, &["c".into(), "a".into(), "b".into()], |it| {
            it.id.as_str()
        });
        assert!(changed);
        assert_eq!(
            items.into_iter().map(|t| t.id).collect::<Vec<_>>(),
            vec!["c", "a", "b"]
        );
    }
}
