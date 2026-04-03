use leptos::prelude::*;

use crate::api;
use crate::api::client::GlooClient;
use crate::state::item_mutations::{
    ItemDateField, apply_date_change_to_items, build_date_update_request, run_optimistic_mutation,
};
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
    client: GlooClient,
    items: RwSignal<Vec<Item>>,
    list_id: String,
    on_error: Option<WriteSignal<Option<String>>>,
) -> ItemActions {
    let lid_add = list_id.clone();
    let client_add = client.clone();
    let on_add = Callback::new(move |req: CreateItemRequest| {
        let lid = lid_add.clone();
        let client = client_add.clone();
        leptos::task::spawn_local(async move {
            match api::create_item(&client, &lid, &req).await {
                Ok(item) => items.update(|list| list.push(item)),
                Err(e) => {
                    if let Some(set_err) = on_error {
                        set_err.set(Some(e.to_string()));
                    }
                }
            }
        });
    });

    let lid_toggle = list_id.clone();
    let client_toggle = client.clone();
    let on_toggle = Callback::new(move |item_id: String| {
        let current_items = items.get_untracked();
        let (_next_items, new_completed) =
            crate::state::transforms::with_item_toggled(&current_items, &item_id);
        let Some(new_completed) = new_completed else {
            return;
        }; // item not found — skip

        let lid = lid_toggle.clone();
        let client = client_toggle.clone();
        let item_id_for_mutation = item_id.clone();
        let item_id_for_request = item_id.clone();
        let set_err = on_error;
        run_optimistic_mutation(
            items,
            move |list| {
                let (next_items, changed_completed) =
                    crate::state::transforms::with_item_toggled(list, &item_id_for_mutation);
                if changed_completed.is_none() {
                    return false;
                }
                *list = next_items;
                true
            },
            move || async move {
                let body = UpdateItemRequest {
                    completed: Some(new_completed),
                    ..Default::default()
                };
                api::update_item(&client, &lid, &item_id_for_request, &body)
                    .await
                    .map(|_| ())
            },
            move |e| {
                if let Some(set_err) = set_err {
                    set_err.set(Some(e.to_string()));
                }
            },
        );
    });

    let lid_delete = list_id.clone();
    let client_delete = client.clone();
    let on_delete = Callback::new(move |item_id: String| {
        let lid = lid_delete.clone();
        let client = client_delete.clone();
        let item_id_for_mutation = item_id.clone();
        let item_id_for_request = item_id.clone();
        let set_err = on_error;
        run_optimistic_mutation(
            items,
            move |list| {
                let next_items =
                    crate::state::transforms::without_item(list, &item_id_for_mutation);
                if next_items.len() == list.len() {
                    return false;
                }
                *list = next_items;
                true
            },
            move || async move { api::delete_item(&client, &lid, &item_id_for_request).await },
            move |e| {
                if let Some(set_err) = set_err {
                    set_err.set(Some(e.to_string()));
                }
            },
        );
    });

    let lid_desc = list_id.clone();
    let client_desc = client.clone();
    let on_description_save = Callback::new(move |(item_id, new_desc): (String, String)| {
        let lid = lid_desc.clone();
        let client = client_desc.clone();
        let next_description = if new_desc.is_empty() {
            None
        } else {
            Some(new_desc.clone())
        };
        let item_id_for_mutation = item_id.clone();
        let item_id_for_request = item_id.clone();
        let set_err = on_error;
        run_optimistic_mutation(
            items,
            move |list| {
                let Some(item) = list.iter_mut().find(|item| item.id == item_id_for_mutation)
                else {
                    return false;
                };
                item.description = next_description.clone();
                true
            },
            move || async move {
                let req = UpdateItemRequest {
                    description: Some(if new_desc.is_empty() {
                        None
                    } else {
                        Some(new_desc)
                    }),
                    ..Default::default()
                };
                api::update_item(&client, &lid, &item_id_for_request, &req)
                    .await
                    .map(|_| ())
            },
            move |e| {
                if let Some(set_err) = set_err {
                    set_err.set(Some(e.to_string()));
                }
            },
        );
    });

    let lid_qty = list_id.clone();
    let client_qty = client.clone();
    let on_quantity_change = Callback::new(move |(item_id, new_actual): (String, i32)| {
        let lid = lid_qty.clone();
        let iid = item_id.clone();
        let client = client_qty.clone();
        let set_err = on_error;
        run_optimistic_mutation(
            items,
            move |list| {
                let Some(item) = list.iter_mut().find(|item| item.id == item_id) else {
                    return false;
                };
                item.actual_quantity = Some(new_actual);
                if let Some(target) = item.quantity {
                    item.completed = new_actual >= target;
                }
                true
            },
            move || async move {
                let req = UpdateItemRequest {
                    actual_quantity: Some(new_actual),
                    ..Default::default()
                };
                api::update_item(&client, &lid, &iid, &req)
                    .await
                    .map(|_| ())
            },
            move |e| {
                if let Some(set_err) = set_err {
                    set_err.set(Some(e.to_string()));
                }
            },
        );
    });

    let lid_date = list_id.clone();
    let client_date = client.clone();
    let on_date_save = Callback::new(
        move |(item_id, date_type, date_val, time_val): (
            String,
            String,
            String,
            Option<String>,
        )| {
            let Some(field) = ItemDateField::parse(&date_type) else {
                return;
            };
            let Some(req) = build_date_update_request(&date_type, &date_val, time_val.clone())
            else {
                return;
            };

            let lid = lid_date.clone();
            let iid = item_id.clone();
            let client = client_date.clone();
            let item_id_for_mutation = item_id.clone();
            let time_for_mutation = time_val.clone();
            let set_err = on_error;
            run_optimistic_mutation(
                items,
                move |list| {
                    apply_date_change_to_items(
                        list,
                        &item_id_for_mutation,
                        field,
                        &date_val,
                        time_for_mutation.as_deref(),
                    )
                },
                move || async move {
                    api::update_item(&client, &lid, &iid, &req)
                        .await
                        .map(|_| ())
                },
                move |e| {
                    if let Some(set_err) = set_err {
                        set_err.set(Some(e.to_string()));
                    }
                },
            );
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
