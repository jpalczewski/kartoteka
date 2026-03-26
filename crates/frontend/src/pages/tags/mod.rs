pub mod detail;

use crate::api;
use crate::components::add_input::AddInput;
use crate::components::tag_tree::{TagTreeRow, build_tag_tree};
use kartoteka_shared::{CreateTagRequest, Tag};
use leptos::prelude::*;

#[component]
pub fn TagsPage() -> impl IntoView {
    if !api::is_logged_in() {
        return view! { <p><a href="/login">"Zaloguj się"</a></p> }.into_any();
    }

    let tags = RwSignal::new(Vec::<Tag>::new());
    let (loading, set_loading) = signal(true);
    let (new_color, set_new_color) = signal("#e94560".to_string());

    leptos::task::spawn_local(async move {
        if let Ok(fetched) = api::fetch_tags().await {
            tags.set(fetched);
        }
        set_loading.set(false);
    });

    let on_create_root = Callback::new(move |name: String| {
        let color = new_color.get_untracked();
        leptos::task::spawn_local(async move {
            let req = CreateTagRequest {
                name,
                color,
                parent_tag_id: None,
            };
            if let Ok(tag) = api::create_tag(&req).await {
                tags.update(|t| t.push(tag));
            }
        });
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-6">"Tagi"</h2>

            <div class="flex gap-2 items-center mb-4">
                <input
                    type="color"
                    aria-label="Kolor tagu"
                    class="w-8 h-8 rounded cursor-pointer border-0 p-0"
                    prop:value=move || new_color.get()
                    on:input=move |ev| set_new_color.set(event_target_value(&ev))
                />
                <AddInput placeholder="Nowy tag..." button_label="Dodaj" on_submit=on_create_root />
            </div>

            {move || {
                if loading.get() {
                    return view! { <p>"Wczytywanie..."</p> }.into_any();
                }
                let all_tags = tags.get();
                if all_tags.is_empty() {
                    return view! { <p class="text-center text-base-content/50 py-12">"Brak tagów. Dodaj pierwszy!"</p> }.into_any();
                }

                let tree = build_tag_tree(&all_tags);
                view! {
                    <div>
                        {tree.into_iter().map(|node| {
                            view! {
                                <TagTreeRow
                                    node=node
                                    depth=0
                                    tags=tags
                                    new_color=new_color
                                />
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </div>
    }
    .into_any()
}
