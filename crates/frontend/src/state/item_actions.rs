//! Item mutations shared by every page that renders item rows.
//!
//! Each action runs its server fn through [`mutate`], so the listed topics are
//! invalidated on success and errors become toasts. Pages take the bundle from
//! [`use_item_actions`] instead of wiring the same server call again.

use kartoteka_shared::ChangeTopic;
use leptos::prelude::*;

use crate::server_fns::items::{
    delete_item, move_item, toggle_item, update_actual_quantity, update_item_dates,
    update_item_description,
};
use crate::server_fns::tags::{assign_tag_to_item, remove_tag_from_item};
use crate::state::invalidation::mutate;

/// `(item_id, start_date, start_time, deadline, deadline_time, hard_deadline)` — times are
/// independent of their dates so the caller can clear/update either half separately.
pub type DateSavePayload = (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// Item mutation callbacks, all wired through [`mutate`].
#[derive(Clone, Copy)]
pub struct ItemActions {
    /// Toggle completion. Invalidates `Items`.
    pub toggle: Callback<String>,
    /// Delete an item. Invalidates `Items` and `Comments`.
    pub delete: Callback<String>,
    /// `(item_id, description)`. Invalidates `Items`.
    pub save_description: Callback<(String, String)>,
    /// `(item_id, actual_quantity)`. Invalidates `Items`.
    pub set_actual_quantity: Callback<(String, i32)>,
    /// `(item_id, target_list_id)`. Invalidates `Items`.
    pub move_to_list: Callback<(String, String)>,
    /// Save start/deadline dates and times. Invalidates `Items`.
    pub save_dates: Callback<DateSavePayload>,
}

/// Build the [`ItemActions`] bundle. Call it in a component body, not in a closure.
pub fn use_item_actions() -> ItemActions {
    ItemActions {
        toggle: Callback::new(|item_id: String| {
            mutate(toggle_item(item_id), &[ChangeTopic::Items]);
        }),
        delete: Callback::new(|item_id: String| {
            mutate(
                delete_item(item_id),
                &[ChangeTopic::Items, ChangeTopic::Comments],
            );
        }),
        save_description: Callback::new(|(item_id, desc): (String, String)| {
            mutate(
                update_item_description(item_id, desc),
                &[ChangeTopic::Items],
            );
        }),
        set_actual_quantity: Callback::new(|(item_id, new_actual): (String, i32)| {
            mutate(
                update_actual_quantity(item_id, new_actual),
                &[ChangeTopic::Items],
            );
        }),
        move_to_list: Callback::new(|(item_id, target_list_id): (String, String)| {
            mutate(move_item(item_id, target_list_id), &[ChangeTopic::Items]);
        }),
        save_dates: Callback::new(|payload: DateSavePayload| {
            let (item_id, start, start_time, deadline, deadline_time, hard) = payload;
            mutate(
                update_item_dates(item_id, start, start_time, deadline, deadline_time, hard),
                &[ChangeTopic::Items],
            );
        }),
    }
}

/// Assign the tag to the item, or remove it when `is_assigned` is set.
/// The caller decides `is_assigned` from its own copy of the item–tag links.
pub fn toggle_item_tag(item_id: String, tag_id: String, is_assigned: bool) {
    let result = async move {
        if is_assigned {
            remove_tag_from_item(item_id, tag_id).await
        } else {
            assign_tag_to_item(item_id, tag_id).await
        }
    };
    mutate(result, &[ChangeTopic::Items, ChangeTopic::Tags]);
}
