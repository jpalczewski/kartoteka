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

pub fn apply_reorder<T, F>(items: &mut Vec<T>, ordered_ids: &[String], get_id: F) -> bool
where
    T: Clone,
    F: Fn(&T) -> &str,
{
    if items.len() != ordered_ids.len() {
        return false;
    }

    let current_ids: Vec<String> = items.iter().map(|item| get_id(item).to_string()).collect();
    if current_ids == ordered_ids {
        return false;
    }

    let reordered = ordered_ids
        .iter()
        .map(|target_id| items.iter().find(|item| get_id(item) == target_id).cloned())
        .collect::<Option<Vec<_>>>();

    match reordered {
        Some(next_items) => {
            *items = next_items;
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reorder_ids_moves_element_before_target() {
        let ids = vec!["a".into(), "b".into(), "c".into()];
        let reordered = reorder_ids(&ids, "c", Some("b")).expect("reordered ids");
        assert_eq!(reordered, vec!["a", "c", "b"]);
    }

    #[test]
    fn reorder_ids_moves_element_to_end() {
        let ids = vec!["a".into(), "b".into(), "c".into()];
        let reordered = reorder_ids(&ids, "a", None).expect("reordered ids");
        assert_eq!(reordered, vec!["b", "c", "a"]);
    }

    #[test]
    fn reorder_ids_returns_none_when_order_does_not_change() {
        let ids = vec!["a".into(), "b".into(), "c".into()];
        assert!(reorder_ids(&ids, "a", Some("b")).is_none());
    }

    #[test]
    fn apply_reorder_rebuilds_items_in_new_order() {
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct Item {
            id: String,
        }

        let mut items = vec![
            Item { id: "a".into() },
            Item { id: "b".into() },
            Item { id: "c".into() },
        ];

        let changed = apply_reorder(&mut items, &["c".into(), "a".into(), "b".into()], |item| {
            item.id.as_str()
        });

        assert!(changed);
        assert_eq!(
            items.into_iter().map(|item| item.id).collect::<Vec<_>>(),
            vec!["c", "a", "b"]
        );
    }
}
