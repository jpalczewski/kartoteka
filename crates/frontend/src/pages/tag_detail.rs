use crate::api;
use crate::components::tag_badge::TagBadge;
use crate::components::tag_tree::build_breadcrumb;
use kartoteka_shared::Tag;
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;
use std::collections::BTreeMap;

#[component]
pub fn TagDetailPage() -> impl IntoView {
    if !api::is_logged_in() {
        return view! { <p><a href="/login">"Zaloguj sie"</a></p> }.into_any();
    }

    let params = use_params_map();
    let tag_id = move || params.read().get("id").unwrap_or_default();

    let all_tags = RwSignal::new(Vec::<Tag>::new());
    let tag = RwSignal::new(Option::<Tag>::None);
    let items = RwSignal::new(Vec::<serde_json::Value>::new());
    let (loading, set_loading) = signal(true);
    let (recursive, set_recursive) = signal(true);

    // Fetch data reactively — re-runs when tag_id changes (e.g. navigating between subtags)
    let _resource = LocalResource::new(move || {
        let tid = tag_id();
        let rec = recursive.get();
        async move {
            if let Ok(tags) = api::fetch_tags().await {
                tag.set(tags.iter().find(|t| t.id == tid).cloned());
                all_tags.set(tags);
            }
            if let Ok(fetched) = api::fetch_tag_items(&tid, rec).await {
                items.set(fetched);
            }
            set_loading.set(false);
        }
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
                        let tags_for_breadcrumb = all_tags.get();
                        let breadcrumb = build_breadcrumb(&tags_for_breadcrumb, &t.id);
                        let all_items = items.get();

                        // Direct children of this tag
                        let children: Vec<Tag> = tags_for_breadcrumb.iter()
                            .filter(|tag| tag.parent_tag_id.as_deref() == Some(&t.id))
                            .cloned()
                            .collect();

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
                                // Breadcrumb
                                {if breadcrumb.len() > 1 {
                                    view! {
                                        <div class="text-sm text-base-content/50 mb-2 flex items-center gap-1">
                                            {breadcrumb.iter().enumerate().map(|(i, bt)| {
                                                let is_last = i == breadcrumb.len() - 1;
                                                let bt_id = bt.id.clone();
                                                let bt_name = bt.name.clone();
                                                if is_last {
                                                    view! { <span class="font-semibold">{bt_name}</span> }.into_any()
                                                } else {
                                                    view! {
                                                        <span>
                                                            <A href=format!("/tags/{bt_id}") attr:class="link link-hover">{bt_name}</A>
                                                            " > "
                                                        </span>
                                                    }.into_any()
                                                }
                                            }).collect_view()}
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {}.into_any()
                                }}

                                // Tag header
                                <h2 class="text-2xl font-bold mb-4 flex items-center gap-2">
                                    <span
                                        class="inline-block w-4 h-4 rounded-full"
                                        style=format!("background: {color}")
                                    ></span>
                                    {t.name}
                                </h2>

                                // Recursive toggle
                                <label class="flex items-center gap-2 cursor-pointer mb-4">
                                    <input
                                        type="checkbox"
                                        class="toggle toggle-sm toggle-primary"
                                        prop:checked=move || recursive.get()
                                        on:change=move |_| set_recursive.update(|v| *v = !*v)
                                    />
                                    <span class="text-sm">"Uwzględnij podtagi"</span>
                                </label>

                                // Items grouped by list
                                {if groups.is_empty() {
                                    view! {
                                        <p class="text-center text-base-content/50 py-12">
                                            "Brak elementów z tym tagiem"
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

                                // Subtags section
                                {if !children.is_empty() {
                                    view! {
                                        <div class="mt-8">
                                            <h3 class="text-xs text-base-content/50 uppercase tracking-wider mb-2">"Podtagi"</h3>
                                            <div class="flex flex-wrap gap-2">
                                                {children.into_iter().map(|child| {
                                                    let child_id = child.id.clone();
                                                    view! {
                                                        <A href=format!("/tags/{child_id}") attr:class="no-underline">
                                                            <TagBadge tag=child />
                                                        </A>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {}.into_any()
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
