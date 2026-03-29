use kartoteka_shared::Item;

/// Toggle an item's completed status. Returns (new list, new completed value).
/// If item_id not found, returns unchanged list and false.
pub fn with_item_toggled(items: &[Item], item_id: &str) -> (Vec<Item>, bool) {
    let mut new_completed = false;
    let result = items
        .iter()
        .map(|item| {
            if item.id == item_id {
                let toggled = !item.completed;
                new_completed = toggled;
                Item { completed: toggled, ..item.clone() }
            } else {
                item.clone()
            }
        })
        .collect();
    (result, new_completed)
}

/// Remove an item by ID. Returns new list without the item.
pub fn without_item(items: &[Item], item_id: &str) -> Vec<Item> {
    items.iter().filter(|i| i.id != item_id).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_shared::Item;

    fn make_item(id: &str, completed: bool) -> Item {
        Item {
            id: id.to_string(),
            list_id: "list-1".to_string(),
            title: format!("Item {id}"),
            description: None,
            completed,
            position: 0,
            quantity: None,
            actual_quantity: None,
            unit: None,
            start_date: None,
            start_time: None,
            deadline: None,
            deadline_time: None,
            hard_deadline: None,
            created_at: "2026-01-01".to_string(),
            updated_at: "2026-01-01".to_string(),
        }
    }

    #[test]
    fn test_toggle_item() {
        let items = vec![make_item("1", false), make_item("2", true)];
        let (result, new_val) = with_item_toggled(&items, "1");
        assert!(new_val); // was false, now true
        assert!(result[0].completed);
        assert!(result[1].completed); // unchanged
    }

    #[test]
    fn test_toggle_item_idempotent_double() {
        let items = vec![make_item("1", false)];
        let (toggled, _) = with_item_toggled(&items, "1");
        let (back, _) = with_item_toggled(&toggled, "1");
        assert_eq!(back[0].completed, items[0].completed);
    }

    #[test]
    fn test_toggle_missing_id() {
        let items = vec![make_item("1", false)];
        let (result, new_val) = with_item_toggled(&items, "nonexistent");
        assert!(!new_val); // default false
        assert_eq!(result[0].completed, false); // unchanged
    }

    #[test]
    fn test_without_item() {
        let items = vec![make_item("1", false), make_item("2", false)];
        let result = without_item(&items, "1");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "2");
    }

    #[test]
    fn test_without_item_missing_id() {
        let items = vec![make_item("1", false)];
        let result = without_item(&items, "nonexistent");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_without_item_empty() {
        let result = without_item(&[], "1");
        assert!(result.is_empty());
    }
}
