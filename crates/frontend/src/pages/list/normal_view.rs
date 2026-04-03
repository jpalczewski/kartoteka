use leptos::prelude::*;

use crate::components::common::dnd::{
    DragHandleLabel, ItemDragHandleButton, ItemDragShell, ItemDragSurface, ItemDropTargetMarker,
};
use crate::components::items::item_row::ItemRow;
use crate::state::dnd::{DraggedItem, ItemDndState, ItemDropTarget};
use kartoteka_shared::{Item, ItemTagLink, List, Tag};

pub struct NormalViewProps {
    pub list_id: String,
    pub items: Vec<Item>,
    pub tags: Vec<Tag>,
    pub item_tag_links: RwSignal<Vec<ItemTagLink>>,
    pub sublists: Vec<List>,
    pub on_toggle: Callback<String>,
    pub on_delete: Callback<String>,
    pub on_tag_toggle: Callback<(String, String)>,
    pub on_description_save: Callback<(String, String)>,
    pub on_quantity_change: Callback<(String, i32)>,
    pub has_quantity: bool,
    pub on_move: Callback<(String, String)>,
    pub on_date_save: Callback<(String, String, String, Option<String>)>,
    pub deadlines_config: serde_json::Value,
    pub enable_item_dnd: bool,
    pub item_dnd_state: RwSignal<ItemDndState>,
    pub on_item_drop: Callback<ItemDropTarget>,
}

#[allow(clippy::too_many_arguments)]
pub fn render_normal_view(p: NormalViewProps) -> impl IntoView {
    let NormalViewProps {
        list_id,
        items,
        tags,
        item_tag_links,
        sublists,
        on_toggle,
        on_delete,
        on_tag_toggle,
        on_description_save,
        on_quantity_change,
        has_quantity,
        on_move,
        on_date_save,
        deadlines_config,
        enable_item_dnd,
        item_dnd_state,
        on_item_drop,
    } = p;
    let move_targets: Vec<(String, String)> = sublists
        .iter()
        .map(|s| (s.id.clone(), s.name.clone()))
        .collect();

    view! {
        <div>
            {items.into_iter().map(|item| {
                let item_id = item.id.clone();
                let dragged_item = DraggedItem {
                    item_id: item.id.clone(),
                    source_list_id: list_id.clone(),
                };
                let dragged_item_for_handle = dragged_item.clone();
                let dragged_item_for_shell = dragged_item.clone();
                let dragged_item_for_surface = dragged_item.clone();
                let drop_target = ItemDropTarget::before(list_id.clone(), item.id.clone());
                let drop_target_for_marker = drop_target.clone();
                let drop_target_for_surface = drop_target.clone();
                let item_tags: Vec<String> = item_tag_links.read().iter()
                    .filter(|l| l.item_id == item.id)
                    .map(|l| l.tag_id.clone())
                    .collect();
                let tags_clone = tags.clone();
                let item_tag_toggle = Callback::new(move |tag_id: String| {
                    on_tag_toggle.run((item_id.clone(), tag_id));
                });
                let mt = move_targets.clone();
                let dc = deadlines_config.clone();
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
                        <ItemDragShell dnd_state=item_dnd_state dragged_item=dragged_item_for_shell>
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
                                    item=item.clone()
                                    on_toggle=on_toggle
                                    on_delete=on_delete
                                    all_tags=tags_clone
                                    item_tag_ids=item_tags
                                    on_tag_toggle=item_tag_toggle
                                    on_description_save=on_description_save
                                    has_quantity=has_quantity
                                    on_quantity_change=on_quantity_change
                                    move_targets=mt
                                    on_move=on_move
                                    on_date_save=on_date_save
                                    deadlines_config=dc
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
                        target=ItemDropTarget::end(list_id)
                        on_drop=on_item_drop
                    />
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }
}
