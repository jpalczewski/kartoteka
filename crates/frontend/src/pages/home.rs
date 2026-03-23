use leptos::prelude::*;

use crate::api;
use crate::components::list_card::ListCard;
use kartoteka_shared::{CreateListRequest, ListType};

#[component]
pub fn HomePage() -> impl IntoView {
    let (new_name, set_new_name) = signal(String::new());
    let (refresh, set_refresh) = signal(0u32);

    let lists = LocalResource::new(move || {
        let _ = refresh.get();
        api::fetch_lists()
    });

    let on_create = move |_| {
        let name = new_name.get();
        if name.trim().is_empty() {
            return;
        }
        set_new_name.set(String::new());
        leptos::task::spawn_local(async move {
            let req = CreateListRequest {
                name,
                list_type: ListType::Custom,
            };
            let _ = api::create_list(&req).await;
            set_refresh.update(|n| *n += 1);
        });
    };

    view! {
        <h2 style="margin: 1rem 0;">"Twoje listy"</h2>

        <div class="input-row">
            <input
                type="text"
                placeholder="Nazwa nowej listy..."
                prop:value=new_name
                on:input=move |ev| set_new_name.set(event_target_value(&ev))
            />
            <button class="btn" on:click=on_create>"Dodaj"</button>
        </div>

        <Suspense fallback=|| view! { <p>"Wczytywanie..."</p> }>
            {move || {
                lists.get().map(|result| {
                    match &*result {
                        Ok(lists) if lists.is_empty() => {
                            view! { <div class="empty-state">"Brak list. Utwórz pierwszą!"</div> }.into_any()
                        }
                        Ok(lists) => {
                            view! {
                                <div>
                                    {lists.iter().map(|l| view! { <ListCard list=l.clone()/> }).collect::<Vec<_>>()}
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
