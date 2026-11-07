//! Keyed rows of one list, shared by the list page and its sublists.

use kartoteka_shared::types::Item;
use leptos::prelude::*;
use reactive_stores::Store;

use super::item_row::{ItemRow, RowTags};
use super::item_view::ItemCaps;
use crate::components::common::dnd::ItemDropTargetMarker;
use crate::state::dnd::{ItemDndState, ItemDropTarget};
use crate::state::item_store::ItemRows;

/// Drag-and-drop wiring of a list: the drag state, and what to do when an item is dropped
/// in front of a row or at the end of the list.
#[derive(Clone, Copy)]
pub struct ItemDnd {
    pub state: RwSignal<ItemDndState>,
    pub on_drop: Callback<ItemDropTarget>,
}

/// One [`ItemRow`] per id in `ids`, keyed by id, so a row keeps its state while the list
/// around it changes. With `dnd`, drop markers sit in front of every row and at the end
/// (`list_id` is the list a drop lands in). The caller provides the flex container.
#[component]
pub fn ItemRowList(
    store: Store<ItemRows<Item>>,
    #[prop(into)] ids: Signal<Vec<String>>,
    #[prop(into)] list_id: Signal<String>,
    #[prop(into)] caps: Signal<ItemCaps>,
    #[prop(into)] move_targets: Signal<Vec<(String, String)>>,
    #[prop(optional_no_strip)] tags: Option<RowTags>,
    #[prop(optional)] dnd: Option<ItemDnd>,
) -> impl IntoView {
    view! {
        <For each=move || ids.get() key=|id| id.clone() let:id>
            {
                let before = dnd.map(|dnd| view! {
                    <ItemDropTargetMarker
                        dnd_state=dnd.state
                        target=ItemDropTarget::before(list_id.get_untracked(), id.clone())
                        on_drop=dnd.on_drop
                    />
                });
                view! {
                    {before}
                    <ItemRow
                        store=store
                        id=id
                        caps=caps
                        move_targets=move_targets
                        tags=tags
                        dnd_state=dnd.map(|dnd| dnd.state)
                    />
                }
            }
        </For>
        {move || dnd.map(|dnd| view! {
            <ItemDropTargetMarker
                dnd_state=dnd.state
                target=ItemDropTarget::end(list_id.get())
                on_drop=dnd.on_drop
                label="Upuść na koniec"
            />
        })}
    }
}
