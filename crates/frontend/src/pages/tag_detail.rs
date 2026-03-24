use kartoteka_shared::Tag;
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use std::collections::BTreeMap;

use crate::api;

#[component]
pub fn TagDetailPage() -> impl IntoView {
    if !api::is_logged_in() {
        return view! { <p><a href="/login">"Zaloguj sie"</a></p> }.into_any();
    }

    let params = use_params_map();
    let tag_id = move || params.read().get("id").unwrap_or_default();

    let tag = RwSignal::new(Option::<Tag>::None);
    let items = RwSignal::new(Vec::<serde_json::Value>::new());
    let (loading, set_loading) = signal(true);

    let tid = tag_id();
    leptos::task::spawn_local(async move {
        if let Ok(tags) = api::fetch_tags().await {
            tag.set(tags.into_iter().find(|t| t.id == tid));
        }
        if let Ok(fetched) = api::fetch_tag_items(&tid).await {
            items.set(fetched);
        }
        set_loading.set(false);
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            {move || {
                if loading.get() {
                    return view! { <p>"Wczytywanie..."</p> }.into_any();
                }
                match tag.get() {
                    None => view! { <p>"Nie znaleziono tagu"</p> }.into_any(),
                    Some(t) => {
                        let color = t.color.clone();
                        let all_items = items.get();

                        // Group items by list_name
                        let mut groups: BTreeMap<(String, String), Vec<serde_json::Value>> = BTreeMap::new();
                        for item in all_items {
                            let list_name = item.get("list_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("(bez listy)")
                                .to_string();
                            let list_id = item.get("list_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            groups.entry((list_id, list_name)).or_default().push(item);
                        }

                        view! {
                            <div>
                                <h2 class="text-2xl font-bold mb-6 flex items-center gap-2">
                                    <span
                                        class="inline-block w-4 h-4 rounded-full"
                                        style=format!("background: {color}")
                                    ></span>
                                    {t.name}
                                </h2>

                                {if groups.is_empty() {
                                    view! {
                                        <p class="text-center text-base-content/50 py-12">
                                            "Brak elementow z tym tagiem"
                                        </p>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div>
                                            {groups.into_iter().map(|((list_id, list_name), group_items)| {
                                                view! {
                                                    <div class="mb-6">
                                                        <h4 class="text-sm font-semibold uppercase tracking-wide mb-2 text-base-content/70">
                                                            <A href=format!("/lists/{list_id}") attr:class="link link-hover">
                                                                {list_name}
                                                            </A>
                                                        </h4>
                                                        {group_items.into_iter().map(|item| {
                                                            let title = item.get("title")
                                                                .and_then(|v| v.as_str())
                                                                .unwrap_or("")
                                                                .to_string();
                                                            let completed = item.get("completed")
                                                                .map(|v| v.as_f64().unwrap_or(0.0) != 0.0 || v.as_bool().unwrap_or(false))
                                                                .unwrap_or(false);
                                                            view! {
                                                                <div class="flex items-center gap-2 py-1 pl-2">
                                                                    <span class=if completed { "text-base-content/40" } else { "" }>
                                                                        {if completed { "\u{2611}" } else { "\u{2610}" }}
                                                                    </span>
                                                                    <span class=if completed { "line-through text-base-content/40" } else { "" }>
                                                                        {title}
                                                                    </span>
                                                                </div>
                                                            }
                                                        }).collect::<Vec<_>>()}
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_any()
                                }}
                            </div>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
    .into_any()
}
