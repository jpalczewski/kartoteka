use leptos::prelude::*;
use leptos_fluent::move_tr;

use crate::api;
use crate::api::client::GlooClient;
use crate::components::common::dnd::{
    DragGrip, drag_handle_class, drag_shell_class, drag_surface_class, drop_marker_class,
    drop_marker_label_class, drop_marker_line_class,
};
use crate::components::items::add_item_input::AddItemInput;
use crate::components::items::item_actions::create_item_actions;
use crate::components::items::item_row::ItemRow;
use crate::state::item_mutations::run_optimistic_mutation;
use crate::state::reorder::{apply_reorder, reorder_ids};
use kartoteka_shared::{Item, ItemTagLink, List, ReorderItemsRequest, Tag};

#[component]
pub fn SublistSection(
    sublist: List,
    #[prop(default = false)] has_quantity: bool,
    #[prop(default = serde_json::Value::Null)] deadlines_config: serde_json::Value,
    #[prop(default = vec![])] all_tags: Vec<Tag>,
    #[prop(default = vec![])] item_tag_links: Vec<ItemTagLink>,
    on_tag_toggle: Callback<(String, String)>,
    #[prop(default = vec![])] move_targets: Vec<(String, String)>,
    /// Called when an item is moved OUT of this sublist: (moved_item, target_list_id)
    #[prop(optional)]
    on_item_moved_out: Option<Callback<(Item, String)>>,
) -> impl IntoView {
    let client = use_context::<GlooClient>().expect("GlooClient not provided");
    let expanded = RwSignal::new(true);
    let items = RwSignal::new(Vec::<Item>::new());
    let (loading, set_loading) = signal(true);
    let dragged_item_id = RwSignal::new(Option::<String>::None);

    let sublist_id = sublist.id.clone();
    let sublist_name = sublist.name.clone();

    // Fetch items on mount
    {
        let sid = sublist_id.clone();
        let client_fetch = client.clone();
        leptos::task::spawn_local(async move {
            if let Ok(fetched) = api::fetch_items(&client_fetch, &sid).await {
                items.set(fetched);
            }
            set_loading.set(false);
        });
    }

    let actions = create_item_actions(client.clone(), items, sublist_id.clone(), None);
    let on_add = actions.on_add;
    let on_toggle = actions.on_toggle;
    let on_delete = actions.on_delete;
    let on_description_save = actions.on_description_save;
    let on_quantity_change = actions.on_quantity_change;

    let move_targets = StoredValue::new(move_targets);

    // Move item callback
    let on_move = Callback::new(move |(item_id, target_list_id): (String, String)| {
        // Find and remove the item, notify parent
        let moved_item = items.read().iter().find(|i| i.id == item_id).cloned();
        items.update(|list| list.retain(|i| i.id != item_id));
        if let Some(mut item) = moved_item {
            item.list_id = target_list_id.clone();
            if let Some(cb) = on_item_moved_out {
                cb.run((item, target_list_id.clone()));
            }
        }
        let client_move = client.clone();
        leptos::task::spawn_local(async move {
            let _ = api::move_item(&client_move, &item_id, &target_list_id).await;
        });
    });

    let on_reorder_drop = Callback::new(move |before_id: Option<String>| {
        let Some(dragged_id) = dragged_item_id.get_untracked() else {
            return;
        };
        let current_ids: Vec<String> = items
            .get_untracked()
            .into_iter()
            .map(|item| item.id)
            .collect();
        let Some(next_ids) = reorder_ids(&current_ids, &dragged_id, before_id.as_deref()) else {
            dragged_item_id.set(None);
            return;
        };

        let request = ReorderItemsRequest {
            item_ids: next_ids.clone(),
        };
        let dragged_id_for_mutation = dragged_id.clone();
        let before_id_for_mutation = before_id.clone();
        let client = client.clone();
        let sublist_id_for_request = sublist_id.clone();
        run_optimistic_mutation(
            items,
            move |items| {
                let current_ids: Vec<String> = items.iter().map(|item| item.id.clone()).collect();
                let Some(next_ids) = reorder_ids(
                    &current_ids,
                    &dragged_id_for_mutation,
                    before_id_for_mutation.as_deref(),
                ) else {
                    return false;
                };
                apply_reorder(items, &next_ids, |item| item.id.as_str())
            },
            move || async move { api::reorder_items(&client, &sublist_id_for_request, &request).await },
            |_| {},
        );
        dragged_item_id.set(None);
    });

    let sorted_items = move || {
        let mut list = items.get();
        list.sort_by(|a, b| a.position.cmp(&b.position));
        list
    };

    // Progress counter
    let progress = move || {
        let list = items.read();
        let total = list.len();
        let completed = list.iter().filter(|i| i.completed).count();
        (completed, total)
    };

    view! {
        <div class="collapse collapse-arrow bg-base-200 mb-2">
            <input
                type="checkbox"
                checked=true
                on:change=move |_| expanded.update(|e| *e = !*e)
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
                {move || {
                    if loading.get() {
                        view! { <p class="text-sm text-base-content/50">{move_tr!("common-loading")}</p> }.into_any()
                    } else {
                        let all_tags_clone = all_tags.clone();
                        let item_tag_links_clone = item_tag_links.clone();
                        view! {
                            <div>
                                {move || sorted_items().iter().map(|item| {
                                    let item_id = item.id.clone();
                                    let drop_before_id = item.id.clone();
                                    let drag_id = item.id.clone();
                                    let drag_id_for_drag = drag_id.clone();
                                    let drag_id_for_shell = drag_id.clone();
                                    let drag_id_for_surface = drag_id.clone();
                                    let item_tags: Vec<String> = item_tag_links_clone.iter()
                                        .filter(|l| l.item_id == item.id)
                                        .map(|l| l.tag_id.clone())
                                        .collect();
                                    let tags_clone = all_tags_clone.clone();
                                    let tog_cb = on_tag_toggle;
                                    let item_tag_toggle = Callback::new(move |tag_id: String| {
                                        tog_cb.run((item_id.clone(), tag_id));
                                    });
                                    let mt = move_targets.get_value();
                                    view! {
                                        <div class="flex flex-col gap-2">
                                            <div
                                                class=move || drop_marker_class(dragged_item_id.get().is_some())
                                                on:dragover=move |ev: web_sys::DragEvent| {
                                                    ev.prevent_default();
                                                    if let Some(data_transfer) = ev.data_transfer() {
                                                        data_transfer.set_drop_effect("move");
                                                    }
                                                }
                                                on:drop=move |ev: web_sys::DragEvent| {
                                                    ev.prevent_default();
                                                    on_reorder_drop.run(Some(drop_before_id.clone()));
                                                }
                                            >
                                                <span class=move || drop_marker_line_class(dragged_item_id.get().is_some())></span>
                                                <span class=move || drop_marker_label_class(dragged_item_id.get().is_some())>"Upuść tutaj"</span>
                                                <span class=move || drop_marker_line_class(dragged_item_id.get().is_some())></span>
                                            </div>
                                            <div class=move || drag_shell_class(
                                                dragged_item_id.get().as_deref() == Some(drag_id_for_shell.as_str())
                                            )>
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
                                                    on:dragend=move |_| dragged_item_id.set(None)
                                                >
                                                    <DragGrip />
                                                </button>
                                                <div class=move || drag_surface_class(
                                                    dragged_item_id.get().as_deref() == Some(drag_id_for_surface.as_str())
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
                                                    />
                                                </div>
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                                <div
                                    class=move || drop_marker_class(dragged_item_id.get().is_some())
                                    on:dragover=move |ev: web_sys::DragEvent| {
                                        ev.prevent_default();
                                        if let Some(data_transfer) = ev.data_transfer() {
                                            data_transfer.set_drop_effect("move");
                                        }
                                    }
                                    on:drop=move |ev: web_sys::DragEvent| {
                                        ev.prevent_default();
                                        on_reorder_drop.run(None);
                                    }
                                >
                                    <span class=move || drop_marker_line_class(dragged_item_id.get().is_some())></span>
                                    <span class=move || drop_marker_label_class(dragged_item_id.get().is_some())>"Upuść na końcu"</span>
                                    <span class=move || drop_marker_line_class(dragged_item_id.get().is_some())></span>
                                </div>
                                <div class="mt-2">
                                    <AddItemInput on_submit=on_add has_quantity=has_quantity deadlines_config=deadlines_config.clone() />
                                </div>
                            </div>
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}
