use leptos::prelude::*;

use crate::api;
use kartoteka_shared::{CreateItemRequest, Item, UpdateItemRequest};

/// All item-level callbacks for a list or sublist.
pub struct ItemActions {
    pub on_add: Callback<CreateItemRequest>,
    pub on_toggle: Callback<String>,
    pub on_delete: Callback<String>,
    pub on_description_save: Callback<(String, String)>,
    pub on_quantity_change: Callback<(String, i32)>,
    /// (item_id, date_type, date_value, time_value)
    /// date_type: "start" | "deadline" | "hard_deadline"
    /// date_value: "" to clear, "YYYY-MM-DD" to set
    pub on_date_save: Callback<(String, String, String, Option<String>)>,
}

/// Create item callbacks bound to a specific list/sublist.
pub fn create_item_actions(
    items: RwSignal<Vec<Item>>,
    list_id: String,
    on_error: Option<WriteSignal<Option<String>>>,
) -> ItemActions {
    let lid_add = list_id.clone();
    let on_add = Callback::new(move |req: CreateItemRequest| {
        let lid = lid_add.clone();
        leptos::task::spawn_local(async move {
            match api::create_item(&lid, &req).await {
                Ok(item) => items.update(|list| list.push(item)),
                Err(e) => {
                    if let Some(set_err) = on_error {
                        set_err.set(Some(e));
                    }
                }
            }
        });
    });

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
                    start_date: None,
                    start_time: None,
                    deadline: None,
                    deadline_time: None,
                    hard_deadline: None,
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
                start_date: None,
                start_time: None,
                deadline: None,
                deadline_time: None,
                hard_deadline: None,
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
                start_date: None,
                start_time: None,
                deadline: None,
                deadline_time: None,
                hard_deadline: None,
            };
            let _ = api::update_item(&lid, &iid, &req).await;
        });
    });

    let lid_date = list_id.clone();
    let on_date_save = Callback::new(
        move |(item_id, date_type, date_val, time_val): (
            String,
            String,
            String,
            Option<String>,
        )| {
            let date_opt = if date_val.is_empty() {
                Some(None) // clear
            } else {
                Some(Some(date_val.clone()))
            };
            let time_opt = if date_val.is_empty() {
                Some(None) // clear time too
            } else {
                time_val.clone().map(Some) // Some(Some("HH:MM")) or None (don't change)
            };

            // Optimistic update
            items.update(|list| {
                if let Some(item) = list.iter_mut().find(|i| i.id == item_id) {
                    let d = if date_val.is_empty() {
                        None
                    } else {
                        Some(date_val.clone())
                    };
                    match date_type.as_str() {
                        "start" => {
                            item.start_date = d;
                            item.start_time = time_val.clone();
                        }
                        "deadline" => {
                            item.deadline = d;
                            item.deadline_time = time_val.clone();
                        }
                        "hard_deadline" => {
                            item.hard_deadline = d;
                        }
                        _ => {}
                    }
                }
            });

            let lid = lid_date.clone();
            let iid = item_id.clone();
            let dt = date_type.clone();
            leptos::task::spawn_local(async move {
                let mut req = UpdateItemRequest {
                    title: None,
                    description: None,
                    completed: None,
                    position: None,
                    quantity: None,
                    actual_quantity: None,
                    unit: None,
                    start_date: None,
                    start_time: None,
                    deadline: None,
                    deadline_time: None,
                    hard_deadline: None,
                };
                match dt.as_str() {
                    "start" => {
                        req.start_date = date_opt;
                        req.start_time = time_opt;
                    }
                    "deadline" => {
                        req.deadline = date_opt;
                        req.deadline_time = time_opt;
                    }
                    "hard_deadline" => {
                        req.hard_deadline = date_opt;
                    }
                    _ => {}
                }
                let _ = api::update_item(&lid, &iid, &req).await;
            });
        },
    );

    ItemActions {
        on_add,
        on_toggle,
        on_delete,
        on_description_save,
        on_quantity_change,
        on_date_save,
    }
}
