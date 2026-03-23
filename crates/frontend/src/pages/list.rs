use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::api;
use crate::components::item_row::ItemRow;
use kartoteka_shared::{CreateItemRequest, UpdateItemRequest};

#[component]
pub fn ListPage() -> impl IntoView {
    let params = use_params_map();
    let list_id = move || params.read().get("id").unwrap_or_default();

    let (new_title, set_new_title) = signal(String::new());
    let (refresh, set_refresh) = signal(0u32);

    let lid = list_id();
    let items = LocalResource::new(move || {
        let lid = lid.clone();
        let _ = refresh.get();
        async move { api::fetch_items(&lid).await }
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
            let _ = api::create_item(&lid, &req).await;
            set_refresh.update(|n| *n += 1);
        });
    };

    let lid_for_toggle = list_id();
    let on_toggle = Callback::new(move |item_id: String| {
        let lid = lid_for_toggle.clone();
        leptos::task::spawn_local(async move {
            let items = api::fetch_items(&lid).await.unwrap_or_default();
            if let Some(item) = items.iter().find(|i| i.id == item_id) {
                let req = UpdateItemRequest {
                    title: None,
                    description: None,
                    completed: Some(!item.completed),
                    position: None,
                };
                let _ = api::update_item(&lid, &item_id, &req).await;
                set_refresh.update(|n| *n += 1);
            }
        });
    });

    let lid_for_delete = list_id();
    let on_delete = Callback::new(move |item_id: String| {
        let lid = lid_for_delete.clone();
        leptos::task::spawn_local(async move {
            let _ = api::delete_item(&lid, &item_id).await;
            set_refresh.update(|n| *n += 1);
        });
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

        <Suspense fallback=|| view! { <p>"Wczytywanie..."</p> }>
            {move || {
                items.get().map(|result| {
                    match &*result {
                        Ok(items) if items.is_empty() => {
                            view! { <div class="empty-state">"Lista jest pusta"</div> }.into_any()
                        }
                        Ok(items) => {
                            view! {
                                <div>
                                    {items.iter().map(|item| {
                                        view! { <ItemRow item=item.clone() on_toggle=on_toggle on_delete=on_delete/> }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any()
                        }
                        Err(e) => {
                            view! { <p style="color: red;">{format!("Błąd: {e}")}</p> }.into_any()
                        }
                    }
                })
            }}
        </Suspense>
    }
}
