use leptos::prelude::*;
use leptos_fluent::move_tr;

use crate::components::common::dnd::{
    DragHandleLabel, ItemDragHandleButton, ItemDragShell, ItemDragSurface, ItemDropTargetMarker,
};
use crate::components::items::add_item_input::AddItemInput;
use crate::components::items::item_row::ItemRow;
use crate::state::dnd::{DraggedItem, ItemDndState, ItemDropTarget};
use kartoteka_shared::{CreateItemRequest, Item, ItemTagLink, List, Tag};

#[component]
#[allow(clippy::too_many_arguments)]
pub fn SublistSection(
    sublist: List,
    items: Vec<Item>,
    #[prop(default = true)] enable_item_dnd: bool,
    item_dnd_state: RwSignal<ItemDndState>,
    on_item_drop: Callback<ItemDropTarget>,
    on_add: Callback<CreateItemRequest>,
    on_toggle: Callback<String>,
    on_delete: Callback<String>,
    on_description_save: Callback<(String, String)>,
    #[prop(default = false)] has_quantity: bool,
    on_quantity_change: Callback<(String, i32)>,
    #[prop(default = serde_json::Value::Null)] deadlines_config: serde_json::Value,
    #[prop(default = vec![])] all_tags: Vec<Tag>,
    #[prop(default = vec![])] item_tag_links: Vec<ItemTagLink>,
    on_tag_toggle: Callback<(String, String)>,
    #[prop(default = vec![])] move_targets: Vec<(String, String)>,
    on_move: Callback<(String, String)>,
    on_date_save: Callback<(String, String, String, Option<String>)>,
) -> impl IntoView {
    let expanded = RwSignal::new(true);
    let sublist_id = sublist.id.clone();
    let sublist_name = sublist.name.clone();
    let items_for_progress = items.clone();

    let progress = move || {
        let total = items_for_progress.len();
        let completed = items_for_progress
            .iter()
            .filter(|item| item.completed)
            .count();
        (completed, total)
    };

    view! {
        <div class="collapse collapse-arrow bg-base-200 mb-2">
            <input
                type="checkbox"
                checked=true
                on:change=move |_| expanded.update(|is_open| *is_open = !*is_open)
            />
            <div class="collapse-title font-semibold flex items-center gap-2">
                <span>{sublist_name}</span>
                <span class="text-sm text-base-content/60 ml-auto mr-4">
                    {move || {
                        let (done, total) = progress();
                        move_tr!("lists-progress", { "done" => done, "total" => total }).get()
                    }}
                </span>
            </div>
            <div class="collapse-content">
                <div>
                    {items.into_iter().map(|item| {
                        let item_id = item.id.clone();
                        let dragged_item = DraggedItem {
                            item_id: item.id.clone(),
                            source_list_id: sublist_id.clone(),
                        };
                        let dragged_item_for_handle = dragged_item.clone();
                        let dragged_item_for_shell = dragged_item.clone();
                        let dragged_item_for_surface = dragged_item.clone();
                        let drop_target = ItemDropTarget::before(sublist_id.clone(), item.id.clone());
                        let drop_target_for_marker = drop_target.clone();
                        let drop_target_for_surface = drop_target.clone();
                        let item_tags: Vec<String> = item_tag_links
                            .iter()
                            .filter(|link| link.item_id == item.id)
                            .map(|link| link.tag_id.clone())
                            .collect();
                        let tags_clone = all_tags.clone();
                        let on_tag_toggle_item = Callback::new(move |tag_id: String| {
                            on_tag_toggle.run((item_id.clone(), tag_id));
                        });
                        let move_targets_for_item = move_targets.clone();
                        let deadlines_config_for_item = deadlines_config.clone();
                        view! {
                            <div class="flex flex-col gap-2">
                                {if enable_item_dnd {
                                    view! {
                                        <ItemDropTargetMarker
                                            dnd_state=item_dnd_state
                                            target=drop_target_for_marker
                                            on_drop=on_item_drop.clone()
                                        />
                                    }.into_any()
                                } else {
                                    view! {}.into_any()
                                }}
                                <ItemDragShell
                                    dnd_state=item_dnd_state
                                    dragged_item=dragged_item_for_shell
                                >
                                    {if enable_item_dnd {
                                        view! {
                                            <ItemDragHandleButton
                                                dnd_state=item_dnd_state
                                                dragged_item=dragged_item_for_handle
                                                label=DragHandleLabel::Reorder
                                                extra_class="mt-2"
                                            />
                                        }.into_any()
                                    } else {
                                        view! {}.into_any()
                                    }}
                                    <ItemDragSurface
                                        dnd_state=item_dnd_state
                                        dragged_item=dragged_item_for_surface
                                        hover_target=drop_target_for_surface
                                    >
                                        <ItemRow
                                            item=item
                                            on_toggle=on_toggle
                                            on_delete=on_delete
                                            all_tags=tags_clone
                                            item_tag_ids=item_tags
                                            on_tag_toggle=on_tag_toggle_item
                                            on_description_save=on_description_save
                                            has_quantity=has_quantity
                                            on_quantity_change=on_quantity_change
                                            move_targets=move_targets_for_item
                                            on_move=on_move
                                            on_date_save=on_date_save
                                            deadlines_config=deadlines_config_for_item
                                        />
                                    </ItemDragSurface>
                                </ItemDragShell>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                    {if enable_item_dnd {
                        view! {
                            <ItemDropTargetMarker
                                dnd_state=item_dnd_state
                                target=ItemDropTarget::end(sublist_id)
                                on_drop=on_item_drop
                            />
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }}
                    <div class="mt-2">
                        <AddItemInput
                            on_submit=on_add
                            has_quantity=has_quantity
                            deadlines_config=deadlines_config
                        />
                    </div>
                </div>
            </div>
        </div>
    }
}
