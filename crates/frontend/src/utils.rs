use kartoteka_shared::types::DateItem;

pub fn group_by_list(items: Vec<DateItem>) -> Vec<(String, String, Vec<DateItem>)> {
    let mut groups: Vec<(String, String, Vec<DateItem>)> = Vec::new();
    for item in items {
        let lid = item.item.list_id.clone();
        if let Some(g) = groups.iter_mut().find(|(id, _, _)| id == &lid) {
            g.2.push(item);
        } else {
            groups.push((lid, item.list_name.clone(), vec![item]));
        }
    }
    groups
}
