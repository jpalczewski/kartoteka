use leptos::prelude::*;

use crate::components::common::dnd::{
    DragGrip, END_DROP_TARGET_ID, drag_handle_class, drag_shell_class, drag_surface_class,
    drop_marker_class, drop_marker_label_class, drop_marker_line_class,
};
use crate::components::items::item_row::ItemRow;
use kartoteka_shared::{Item, ItemTagLink, List, Tag};

pub struct NormalViewProps {
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
    pub enable_reorder: bool,
    pub dragged_item_id: RwSignal<Option<String>>,
    pub on_reorder_drop: Callback<Option<String>>,
}

#[allow(clippy::too_many_arguments)]
pub fn render_normal_view(p: NormalViewProps) -> impl IntoView {
    let NormalViewProps {
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
        enable_reorder,
        dragged_item_id,
        on_reorder_drop,
    } = p;
    let move_targets: Vec<(String, String)> = sublists
        .iter()
        .map(|s| (s.id.clone(), s.name.clone()))
        .collect();
    let hovered_drop_id = RwSignal::new(Option::<String>::None);
    let is_end_drop_target_hovered =
        Signal::derive(move || hovered_drop_id.get().as_deref() == Some(END_DROP_TARGET_ID));

    view! {
        <div>
            {items.iter().map(|item| {
                let item_id = item.id.clone();
                let drop_before_id = item.id.clone();
                let drop_target_id = item.id.clone();
                let drop_target_id_for_dragover = drop_target_id.clone();
                let drop_before_id_for_drop = drop_before_id.clone();
                let drop_target_id_for_hover = drop_target_id.clone();
                let drag_id = item.id.clone();
                let drag_id_for_drag = drag_id.clone();
                let drag_id_for_shell = drag_id.clone();
                let drag_id_for_surface = drag_id.clone();
                let is_drop_target_hovered = Signal::derive(move || {
                    hovered_drop_id.get().as_deref() == Some(drop_target_id_for_hover.as_str())
                });
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
                        {if enable_reorder {
                            view! {
                                <div
                                    class=move || drop_marker_class(
                                        dragged_item_id.get().is_some(),
                                        is_drop_target_hovered.get(),
                                    )
                                    on:dragover=move |ev: web_sys::DragEvent| {
                                        ev.prevent_default();
                                        if let Some(data_transfer) = ev.data_transfer() {
                                            data_transfer.set_drop_effect("move");
                                        }
                                        hovered_drop_id.set(Some(drop_target_id_for_dragover.clone()));
                                    }
                                    on:drop=move |ev: web_sys::DragEvent| {
                                        ev.prevent_default();
                                        hovered_drop_id.set(None);
                                        on_reorder_drop.run(Some(drop_before_id_for_drop.clone()));
                                    }
                                >
                                    <span class=move || drop_marker_line_class(
                                        dragged_item_id.get().is_some(),
                                        is_drop_target_hovered.get(),
                                    )></span>
                                    <span class=move || drop_marker_label_class(
                                        dragged_item_id.get().is_some(),
                                        is_drop_target_hovered.get(),
                                    )>"Upuść tutaj"</span>
                                    <span class=move || drop_marker_line_class(
                                        dragged_item_id.get().is_some(),
                                        is_drop_target_hovered.get(),
                                    )></span>
                                </div>
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}
                        <div class=move || drag_shell_class(
                            dragged_item_id.get().as_deref() == Some(drag_id_for_shell.as_str())
                        )>
                            {if enable_reorder {
                                view! {
                                    <button
                                        type="button"
                                        class=move || format!(
                                            "{} mt-2",
                                            drag_handle_class(
                                                dragged_item_id.get().as_deref() == Some(drag_id.as_str())
                                            )
                                        )
                                        draggable="true"
                                        aria-label="Przeciągnij, aby zmienić kolejność"
                                        title="Przeciągnij, aby zmienić kolejność"
                                        on:dragstart=move |ev: web_sys::DragEvent| {
                                            if let Some(data_transfer) = ev.data_transfer() {
                                                let _ = data_transfer.set_data("text/plain", &drag_id_for_drag);
                                                data_transfer.set_effect_allowed("move");
                                            }
                                            dragged_item_id.set(Some(drag_id_for_drag.clone()));
                                        }
                                        on:dragend=move |_| {
                                            dragged_item_id.set(None);
                                            hovered_drop_id.set(None);
                                        }
                                    >
                                        <DragGrip />
                                    </button>
                                }.into_any()
                            } else {
                                view! {}.into_any()
                            }}
                            <div class=move || drag_surface_class(
                                dragged_item_id.get().as_deref() == Some(drag_id_for_surface.as_str()),
                                is_drop_target_hovered.get(),
                            )>
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
                            </div>
                        </div>
                    </div>
                }
            }).collect::<Vec<_>>()}
            {if enable_reorder {
                view! {
                    <div
                        class=move || drop_marker_class(
                            dragged_item_id.get().is_some(),
                            is_end_drop_target_hovered.get(),
                        )
                        on:dragover=move |ev: web_sys::DragEvent| {
                            ev.prevent_default();
                            if let Some(data_transfer) = ev.data_transfer() {
                                data_transfer.set_drop_effect("move");
                            }
                            hovered_drop_id.set(Some(END_DROP_TARGET_ID.to_string()));
                        }
                        on:drop=move |ev: web_sys::DragEvent| {
                            ev.prevent_default();
                            hovered_drop_id.set(None);
                            on_reorder_drop.run(None);
                        }
                    >
                        <span class=move || drop_marker_line_class(
                            dragged_item_id.get().is_some(),
                            is_end_drop_target_hovered.get(),
                        )></span>
                        <span class=move || drop_marker_label_class(
                            dragged_item_id.get().is_some(),
                            is_end_drop_target_hovered.get(),
                        )>"Upuść na końcu"</span>
                        <span class=move || drop_marker_line_class(
                            dragged_item_id.get().is_some(),
                            is_end_drop_target_hovered.get(),
                        )></span>
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }
}
