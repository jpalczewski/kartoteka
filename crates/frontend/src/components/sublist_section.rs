use leptos::prelude::*;

use crate::api;
use crate::components::add_item_input::AddItemInput;
use crate::components::item_actions::create_item_actions;
use crate::components::item_row::ItemRow;
use kartoteka_shared::{Item, ItemTagLink, List, Tag};

#[component]
pub fn SublistSection(
    sublist: List,
    #[prop(default = false)] has_quantity: bool,
    #[prop(default = false)] has_due_date: bool,
    #[prop(default = vec![])] all_tags: Vec<Tag>,
    #[prop(default = vec![])] item_tag_links: Vec<ItemTagLink>,
    on_tag_toggle: Callback<(String, String)>,
    #[prop(default = vec![])] move_targets: Vec<(String, String)>,
    /// Called when an item is moved OUT of this sublist: (moved_item, target_list_id)
    #[prop(optional)] on_item_moved_out: Option<Callback<(Item, String)>>,
) -> impl IntoView {
    let expanded = RwSignal::new(true);
    let items = RwSignal::new(Vec::<Item>::new());
    let (loading, set_loading) = signal(true);

    let sublist_id = sublist.id.clone();
    let sublist_name = sublist.name.clone();

    // Fetch items on mount
    {
        let sid = sublist_id.clone();
        leptos::task::spawn_local(async move {
            if let Ok(fetched) = api::fetch_items(&sid).await {
                items.set(fetched);
            }
            set_loading.set(false);
        });
    }

    let actions = create_item_actions(items, sublist_id.clone(), None);
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
        leptos::task::spawn_local(async move {
            let _ = api::move_item(&item_id, &target_list_id).await;
        });
    });

    let sorted_items = move || {
        let mut list = items.get();
        list.sort_by(|a, b| {
            a.completed
                .cmp(&b.completed)
                .then(a.position.cmp(&b.position))
        });
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
                        format!("{done}/{total} \u{2713}")
                    }}
                </span>
            </div>
            <div class="collapse-content">
                {move || {
                    if loading.get() {
                        view! { <p class="text-sm text-base-content/50">"Wczytywanie..."</p> }.into_any()
                    } else {
                        let all_tags_clone = all_tags.clone();
                        let item_tag_links_clone = item_tag_links.clone();
                        view! {
                            <div>
                                {move || sorted_items().iter().map(|item| {
                                    let item_id = item.id.clone();
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
                                    }
                                }).collect::<Vec<_>>()}
                                <div class="mt-2">
                                    <AddItemInput on_submit=on_add has_quantity=has_quantity has_due_date=has_due_date />
                                </div>
                            </div>
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}
