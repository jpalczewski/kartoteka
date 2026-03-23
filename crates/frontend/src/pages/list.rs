use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::api;
use crate::components::item_row::ItemRow;
use kartoteka_shared::{CreateItemRequest, Item, ItemTagLink, Tag, UpdateItemRequest};

#[component]
pub fn ListPage() -> impl IntoView {
    let params = use_params_map();
    let list_id = move || params.read().get("id").unwrap_or_default();

    let (new_title, set_new_title) = signal(String::new());
    let items = RwSignal::new(Vec::<Item>::new());
    let all_tags = RwSignal::new(Vec::<Tag>::new());
    let item_tag_links = RwSignal::new(Vec::<ItemTagLink>::new());
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
        set_loading.set(false);
    });

    let lid_for_create = list_id();
    let on_add = move |_| {
        let title = new_title.get();
        if title.trim().is_empty() {
            return;
        }
        set_new_title.set(String::new());
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
    };

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

    // Tag toggle callback with optimistic updates
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

    view! {
        <h2 style="margin: 1rem 0;">"Lista"</h2>

        <div class="input-row">
            <input
                type="text"
                placeholder="Nowy element..."
                prop:value=new_title
                on:input=move |ev| set_new_title.set(event_target_value(&ev))
            />
            <button class="btn" on:click=on_add>"Dodaj"</button>
        </div>

        {move || {
            if loading.get() {
                view! { <p>"Wczytywanie..."</p> }.into_any()
            } else if let Some(e) = error.get() {
                view! { <p style="color: red;">{format!("Błąd: {e}")}</p> }.into_any()
            } else if items.read().is_empty() {
                view! { <div class="empty-state">"Lista jest pusta"</div> }.into_any()
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
    }
}
