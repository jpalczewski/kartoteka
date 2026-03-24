use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::api;
use crate::components::add_input::AddInput;
use crate::components::item_row::ItemRow;
use crate::components::tag_badge::TagBadge;
use crate::components::tag_selector::TagSelector;
use kartoteka_shared::{
    CreateItemRequest, Item, ItemTagLink, ListTagLink, Tag, UpdateItemRequest,
};

#[component]
pub fn ListPage() -> impl IntoView {
    let params = use_params_map();
    let list_id = move || params.read().get("id").unwrap_or_default();

    let items = RwSignal::new(Vec::<Item>::new());
    let all_tags = RwSignal::new(Vec::<Tag>::new());
    let item_tag_links = RwSignal::new(Vec::<ItemTagLink>::new());
    let list_tag_links = RwSignal::new(Vec::<ListTagLink>::new());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(Option::<String>::None);

    // Initial fetch
    let lid = list_id();
    leptos::task::spawn_local(async move {
        match api::fetch_items(&lid).await {
            Ok(fetched) => items.set(fetched),
            Err(e) => set_error.set(Some(e)),
        }
        if let Ok(fetched_tags) = api::fetch_tags().await {
            all_tags.set(fetched_tags);
        }
        if let Ok(links) = api::fetch_item_tag_links().await {
            item_tag_links.set(links);
        }
        if let Ok(links) = api::fetch_list_tag_links().await {
            let filtered: Vec<ListTagLink> =
                links.into_iter().filter(|l| l.list_id == lid).collect();
            list_tag_links.set(filtered);
        }
        set_loading.set(false);
    });

    let lid_for_create = list_id();
    let on_add = Callback::new(move |title: String| {
        let lid = lid_for_create.clone();
        leptos::task::spawn_local(async move {
            let req = CreateItemRequest {
                title,
                description: None,
            };
            match api::create_item(&lid, &req).await {
                Ok(item) => items.update(|list| list.push(item)),
                Err(e) => set_error.set(Some(e)),
            }
        });
    });

    let lid_for_toggle = list_id();
    let on_toggle = Callback::new(move |item_id: String| {
        // Optimistic update
        items.update(|list| {
            if let Some(item) = list.iter_mut().find(|i| i.id == item_id) {
                item.completed = !item.completed;
            }
        });

        let lid = lid_for_toggle.clone();
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
                };
                let _ = api::update_item(&lid, &item_id, &req).await;
            });
        }
    });

    let lid_for_delete = list_id();
    let on_delete = Callback::new(move |item_id: String| {
        // Optimistic update
        items.update(|list| list.retain(|i| i.id != item_id));

        let lid = lid_for_delete.clone();
        leptos::task::spawn_local(async move {
            let _ = api::delete_item(&lid, &item_id).await;
        });
    });

    // Item tag toggle callback with optimistic updates
    let on_tag_toggle = Callback::new(move |(item_id, tag_id): (String, String)| {
        let has_tag = item_tag_links
            .read()
            .iter()
            .any(|l| l.item_id == item_id && l.tag_id == tag_id);
        if has_tag {
            item_tag_links.update(|links| {
                links.retain(|l| !(l.item_id == item_id && l.tag_id == tag_id))
            });
            let iid = item_id.clone();
            let tid = tag_id.clone();
            leptos::task::spawn_local(async move {
                let _ = api::remove_tag_from_item(&iid, &tid).await;
            });
        } else {
            item_tag_links.update(|links| {
                links.push(ItemTagLink {
                    item_id: item_id.clone(),
                    tag_id: tag_id.clone(),
                })
            });
            let iid = item_id.clone();
            let tid = tag_id.clone();
            leptos::task::spawn_local(async move {
                let _ = api::assign_tag_to_item(&iid, &tid).await;
            });
        }
    });

    // List tag toggle callback with optimistic updates
    let lid_for_tag = list_id();
    let on_list_tag_toggle = Callback::new(move |tag_id: String| {
        let has_tag = list_tag_links.read().iter().any(|l| l.tag_id == tag_id);
        if has_tag {
            list_tag_links.update(|links| links.retain(|l| l.tag_id != tag_id));
            let lid = lid_for_tag.clone();
            let tid = tag_id.clone();
            leptos::task::spawn_local(async move {
                let _ = api::remove_tag_from_list(&lid, &tid).await;
            });
        } else {
            let lid = lid_for_tag.clone();
            list_tag_links.update(|links| {
                links.push(ListTagLink {
                    list_id: lid.clone(),
                    tag_id: tag_id.clone(),
                })
            });
            let tid = tag_id.clone();
            leptos::task::spawn_local(async move {
                let _ = api::assign_tag_to_list(&lid, &tid).await;
            });
        }
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-4">"Lista"</h2>

            // List tag management
            {move || {
                let links = list_tag_links.read();
                let tags = all_tags.read();
                let assigned_ids: Vec<String> = links.iter().map(|l| l.tag_id.clone()).collect();
                let assigned: Vec<Tag> = tags
                    .iter()
                    .filter(|t| assigned_ids.contains(&t.id))
                    .cloned()
                    .collect();
                view! {
                    <div class="flex flex-wrap items-center gap-1 mb-3">
                        {assigned.into_iter().map(|t| {
                            let tid = t.id.clone();
                            let cb = on_list_tag_toggle.clone();
                            view! {
                                <TagBadge
                                    tag=t
                                    on_remove=Callback::new(move |_: String| cb.run(tid.clone()))
                                />
                            }
                        }).collect::<Vec<_>>()}
                        {if !tags.is_empty() {
                            view! {
                                <TagSelector
                                    all_tags=tags.clone()
                                    selected_tag_ids=assigned_ids
                                    on_toggle=on_list_tag_toggle
                                />
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}
                    </div>
                }
            }}

            <div class="flex gap-2 mb-4">
                <AddInput placeholder="Nowy element..." button_label="Dodaj" on_submit=on_add />
            </div>

            {move || {
                if loading.get() {
                    view! { <p>"Wczytywanie..."</p> }.into_any()
                } else if let Some(e) = error.get() {
                    view! { <p style="color: red;">{format!("Błąd: {e}")}</p> }.into_any()
                } else if items.read().is_empty() {
                    view! { <div class="text-center text-base-content/50 py-12">"Lista jest pusta"</div> }.into_any()
                } else {
                    view! {
                        <div>
                            {move || items.read().iter().map(|item| {
                                let item_id = item.id.clone();
                                let item_tags: Vec<String> = item_tag_links.read().iter()
                                    .filter(|l| l.item_id == item.id)
                                    .map(|l| l.tag_id.clone())
                                    .collect();
                                let tags_clone = all_tags.get();
                                let tog_cb = on_tag_toggle.clone();
                                let item_tag_toggle = Callback::new(move |tag_id: String| {
                                    tog_cb.run((item_id.clone(), tag_id));
                                });
                                view! {
                                    <ItemRow
                                        item=item.clone()
                                        on_toggle=on_toggle
                                        on_delete=on_delete
                                        all_tags=tags_clone
                                        item_tag_ids=item_tags
                                        on_tag_toggle=item_tag_toggle
                                    />
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
