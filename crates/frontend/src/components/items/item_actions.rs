use leptos::prelude::*;

use crate::api;
use kartoteka_shared::{CreateItemRequest, Item, UpdateItemRequest};

/// All item-level callbacks for a list or sublist.
pub struct ItemActions {
    pub on_add: Callback<(
        String,
        Option<String>,
        Option<i32>,
        Option<String>,
        Option<String>,
        Option<String>,
    )>,
    pub on_toggle: Callback<String>,
    pub on_delete: Callback<String>,
    pub on_description_save: Callback<(String, String)>,
    pub on_quantity_change: Callback<(String, i32)>,
}

/// Create item callbacks bound to a specific list/sublist.
///
/// Both `list.rs` and `sublist_section.rs` use this to avoid duplicating
/// ~100 lines of callback logic.
///
/// When `on_error` is `Some`, creation errors are reported via the signal
/// (used in `list.rs`). When `None`, errors are silently ignored (sublists).
pub fn create_item_actions(
    items: RwSignal<Vec<Item>>,
    list_id: String,
    on_error: Option<WriteSignal<Option<String>>>,
) -> ItemActions {
    let lid_add = list_id.clone();
    let on_add = Callback::new(
        move |(title, description, quantity, unit, due_date, due_time): (
            String,
            Option<String>,
            Option<i32>,
            Option<String>,
            Option<String>,
            Option<String>,
        )| {
            let lid = lid_add.clone();
            leptos::task::spawn_local(async move {
                let req = CreateItemRequest {
                    title,
                    description,
                    quantity,
                    unit,
                    due_date,
                    due_time,
                };
                match api::create_item(&lid, &req).await {
                    Ok(item) => items.update(|list| list.push(item)),
                    Err(e) => {
                        if let Some(set_err) = on_error {
                            set_err.set(Some(e));
                        }
                    }
                }
            });
        },
    );

    let lid_toggle = list_id.clone();
    let on_toggle = Callback::new(move |item_id: String| {
        items.update(|list| {
            if let Some(item) = list.iter_mut().find(|i| i.id == item_id) {
                item.completed = !item.completed;
            }
        });

        let lid = lid_toggle.clone();
        let completed = items
            .read()
            .iter()
            .find(|i| i.id == item_id)
            .map(|i| i.completed);

        if let Some(completed) = completed {
            leptos::task::spawn_local(async move {
                let req = UpdateItemRequest {
                    title: None,
                    description: None,
                    completed: Some(completed),
                    position: None,
                    quantity: None,
                    actual_quantity: None,
                    unit: None,
                    due_date: None,
                    due_time: None,
                };
                let _ = api::update_item(&lid, &item_id, &req).await;
            });
        }
    });

    let lid_delete = list_id.clone();
    let on_delete = Callback::new(move |item_id: String| {
        items.update(|list| list.retain(|i| i.id != item_id));

        let lid = lid_delete.clone();
        leptos::task::spawn_local(async move {
            let _ = api::delete_item(&lid, &item_id).await;
        });
    });

    let lid_desc = list_id.clone();
    let on_description_save = Callback::new(move |(item_id, new_desc): (String, String)| {
        items.update(|list| {
            if let Some(item) = list.iter_mut().find(|i| i.id == item_id) {
                item.description = if new_desc.is_empty() {
                    None
                } else {
                    Some(new_desc.clone())
                };
            }
        });
        let lid = lid_desc.clone();
        leptos::task::spawn_local(async move {
            let req = UpdateItemRequest {
                title: None,
                description: Some(new_desc),
                completed: None,
                position: None,
                quantity: None,
                actual_quantity: None,
                unit: None,
                due_date: None,
                due_time: None,
            };
            let _ = api::update_item(&lid, &item_id, &req).await;
        });
    });

    let lid_qty = list_id.clone();
    let on_quantity_change = Callback::new(move |(item_id, new_actual): (String, i32)| {
        items.update(|list| {
            if let Some(item) = list.iter_mut().find(|i| i.id == item_id) {
                item.actual_quantity = Some(new_actual);
                if let Some(target) = item.quantity {
                    item.completed = new_actual >= target;
                }
            }
        });

        let lid = lid_qty.clone();
        let iid = item_id.clone();
        leptos::task::spawn_local(async move {
            let req = UpdateItemRequest {
                title: None,
                description: None,
                completed: None,
                position: None,
                quantity: None,
                actual_quantity: Some(new_actual),
                unit: None,
                due_date: None,
                due_time: None,
            };
            let _ = api::update_item(&lid, &iid, &req).await;
        });
    });

    ItemActions {
        on_add,
        on_toggle,
        on_delete,
        on_description_save,
        on_quantity_change,
    }
}
