use kartoteka_shared::types::DateItem;

/// The items of one list inside a section, in display order.
#[derive(Clone, Debug, PartialEq)]
pub struct ListGroup {
    pub list_id: String,
    pub list_name: String,
    pub item_ids: Vec<String>,
}

/// Group entries by list, keeping first-seen list order and item order.
pub fn group_ids(items: &[DateItem]) -> Vec<ListGroup> {
    let mut groups: Vec<ListGroup> = Vec::new();
    for di in items {
        match groups.iter_mut().find(|g| g.list_id == di.item.list_id) {
            Some(group) => group.item_ids.push(di.item.id.clone()),
            None => groups.push(ListGroup {
                list_id: di.item.list_id.clone(),
                list_name: di.list_name.clone(),
                item_ids: vec![di.item.id.clone()],
            }),
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_shared::types::Item;

    fn entry(id: &str, list_id: &str, list_name: &str) -> DateItem {
        DateItem {
            item: Item {
                id: id.into(),
                list_id: list_id.into(),
                title: id.into(),
                description: None,
                completed: false,
                status: "todo".into(),
                icon: None,
                position: 0,
                quantity: None,
                actual_quantity: None,
                unit: None,
                start_date: None,
                start_time: None,
                deadline: None,
                deadline_time: None,
                hard_deadline: None,
                estimated_duration: None,
                location_id: None,
                recurrence_rule: None,
                recurrence_mode: None,
                created_at: String::new(),
                updated_at: String::new(),
            },
            list_name: list_name.into(),
            list_type: "checklist".into(),
        }
    }

    #[test]
    fn group_ids_keeps_first_seen_list_order_and_item_order() {
        let groups = group_ids(&[
            entry("a", "l1", "Work"),
            entry("b", "l2", "Home"),
            entry("c", "l1", "Work"),
        ]);
        assert_eq!(
            groups,
            vec![
                ListGroup {
                    list_id: "l1".into(),
                    list_name: "Work".into(),
                    item_ids: vec!["a".into(), "c".into()],
                },
                ListGroup {
                    list_id: "l2".into(),
                    list_name: "Home".into(),
                    item_ids: vec!["b".into()],
                },
            ]
        );
    }
}
