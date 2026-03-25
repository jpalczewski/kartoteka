use crate::api;
use crate::components::editable_color::EditableColor;
use crate::components::editable_title::EditableTitle;
use crate::components::tag_tree::{build_breadcrumb, build_subtree, get_descendant_ids, TagTreeRow};
use kartoteka_shared::{Tag, UpdateTagRequest};
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map};
use std::collections::BTreeMap;

#[derive(Clone, PartialEq)]
enum DetailAction {
    Move,
    Merge,
}

#[component]
pub fn TagDetailPage() -> impl IntoView {
    if !api::is_logged_in() {
        return view! { <p><a href="/login">"Zaloguj sie"</a></p> }.into_any();
    }

    let params = use_params_map();
    let tag_id = move || params.read().get("id").unwrap_or_default();
    let navigate = use_navigate();

    let all_tags = RwSignal::new(Vec::<Tag>::new());
    let tag = RwSignal::new(Option::<Tag>::None);
    let items = RwSignal::new(Vec::<serde_json::Value>::new());
    let (loading, set_loading) = signal(true);
    let (recursive, set_recursive) = signal(true);
    let active_action = RwSignal::new(Option::<DetailAction>::None);
    let (new_color, _set_new_color) = signal("#e94560".to_string());

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
            // Back button
            <A href="/tags" attr:class="text-sm text-base-content/50 hover:text-primary mb-2 inline-block">
                "\u{2190} Wszystkie tagi"
            </A>

            {
                let navigate = navigate.clone();
                move || {
                if loading.get() {
                    return view! { <p>"Wczytywanie..."</p> }.into_any();
                }
                let navigate = navigate.clone();
                match tag.get() {
                    None => view! { <p>"Nie znaleziono tagu"</p> }.into_any(),
                    Some(t) => {
                        let color = t.color.clone();
                        let tags_for_breadcrumb = all_tags.get();
                        let breadcrumb = build_breadcrumb(&tags_for_breadcrumb, &t.id);
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

                        // Build subtree for this tag
                        let subtree = build_subtree(&tags_for_breadcrumb, &t.id);

                        let tag_id_for_move = t.id.clone();
                        let tag_id_for_merge = t.id.clone();
                        let tag_id_for_delete = t.id.clone();

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

                                // Tag header — editable name + color
                                <div class="flex items-center gap-2 mb-4">
                                    {
                                        let tag_id_color = t.id.clone();
                                        view! {
                                            <EditableColor
                                                color=color.clone()
                                                on_save=Callback::new(move |new_color: String| {
                                                    tag.update(|t| {
                                                        if let Some(t) = t { t.color = new_color.clone(); }
                                                    });
                                                    let tid = tag_id_color.clone();
                                                    leptos::task::spawn_local(async move {
                                                        let req = UpdateTagRequest {
                                                            name: None,
                                                            color: Some(new_color),
                                                            parent_tag_id: None,
                                                        };
                                                        let _ = api::update_tag(&tid, &req).await;
                                                    });
                                                })
                                            />
                                        }
                                    }
                                    {
                                        let tag_id_name = t.id.clone();
                                        view! {
                                            <EditableTitle
                                                value=t.name.clone()
                                                on_save=Callback::new(move |new_name: String| {
                                                    tag.update(|t| {
                                                        if let Some(t) = t { t.name = new_name.clone(); }
                                                    });
                                                    let tid = tag_id_name.clone();
                                                    leptos::task::spawn_local(async move {
                                                        let req = UpdateTagRequest {
                                                            name: Some(new_name),
                                                            color: None,
                                                            parent_tag_id: None,
                                                        };
                                                        let _ = api::update_tag(&tid, &req).await;
                                                    });
                                                })
                                            />
                                        }
                                    }
                                </div>

                                // Action buttons
                                <div class="flex gap-1 mb-4">
                                    <button
                                        class="btn btn-ghost btn-xs"
                                        on:click=move |_| {
                                            let current = active_action.get_untracked();
                                            if current == Some(DetailAction::Move) {
                                                active_action.set(None);
                                            } else {
                                                active_action.set(Some(DetailAction::Move));
                                            }
                                        }
                                    >"\u{2195} Przenieś"</button>
                                    <button
                                        class="btn btn-ghost btn-xs"
                                        on:click=move |_| {
                                            let current = active_action.get_untracked();
                                            if current == Some(DetailAction::Merge) {
                                                active_action.set(None);
                                            } else {
                                                active_action.set(Some(DetailAction::Merge));
                                            }
                                        }
                                    >"\u{2295} Scal"</button>
                                    <button
                                        class="btn btn-error btn-xs"
                                        on:click={
                                            let nav = navigate.clone();
                                            let tid = tag_id_for_delete.clone();
                                            move |_| {
                                                let tid = tid.clone();
                                                let nav = nav.clone();
                                                leptos::task::spawn_local(async move {
                                                    let _ = api::delete_tag(&tid).await;
                                                    nav("/tags", Default::default());
                                                });
                                            }
                                        }
                                    >"\u{1F5D1} Usuń"</button>
                                </div>

                                // Action forms
                                {move || {
                                    match active_action.get() {
                                        Some(DetailAction::Move) => {
                                            let tags_snapshot = all_tags.get_untracked();
                                            let tid = tag_id_for_move.clone();
                                            let desc_ids = get_descendant_ids(&tags_snapshot, &tid);
                                            let available: Vec<(String, String)> = tags_snapshot.iter()
                                                .filter(|t| t.id != tid && !desc_ids.contains(&t.id))
                                                .map(|t| (t.id.clone(), t.name.clone()))
                                                .collect();

                                            view! {
                                                <div class="flex gap-2 items-center mb-4">
                                                    <select
                                                        class="select select-bordered select-sm"
                                                        on:change={
                                                            let tid = tid.clone();
                                                            move |ev| {
                                                                let value = event_target_value(&ev);
                                                                let new_parent = if value.is_empty() { None } else { Some(value) };
                                                                let tid = tid.clone();
                                                                leptos::task::spawn_local(async move {
                                                                    let req = UpdateTagRequest {
                                                                        name: None,
                                                                        color: None,
                                                                        parent_tag_id: Some(new_parent),
                                                                    };
                                                                    let _ = api::update_tag(&tid, &req).await;
                                                                    if let Ok(fetched) = api::fetch_tags().await {
                                                                        all_tags.set(fetched);
                                                                    }
                                                                    active_action.set(None);
                                                                });
                                                            }
                                                        }
                                                    >
                                                        <option value="" selected=true>"\u{2014} Brak rodzica \u{2014}"</option>
                                                        {available.into_iter().map(|(id, name)| {
                                                            view! { <option value=id>{name}</option> }
                                                        }).collect_view()}
                                                    </select>
                                                    <button class="btn btn-ghost btn-xs" on:click=move |_| active_action.set(None)>"\u{2715}"</button>
                                                </div>
                                            }.into_any()
                                        }
                                        Some(DetailAction::Merge) => {
                                            let tags_snapshot = all_tags.get_untracked();
                                            let tid = tag_id_for_merge.clone();
                                            let nav = navigate.clone();
                                            let available: Vec<(String, String)> = tags_snapshot.iter()
                                                .filter(|t| t.id != tid)
                                                .map(|t| (t.id.clone(), t.name.clone()))
                                                .collect();

                                            view! {
                                                <div class="flex gap-2 items-center mb-4">
                                                    <select
                                                        class="select select-bordered select-sm"
                                                        on:change={
                                                            let tid = tid.clone();
                                                            let nav = nav.clone();
                                                            move |ev| {
                                                                let value = event_target_value(&ev);
                                                                if value.is_empty() { return; }
                                                                let tid = tid.clone();
                                                                let target = value.clone();
                                                                let nav = nav.clone();
                                                                leptos::task::spawn_local(async move {
                                                                    let _ = api::merge_tag(&tid, &target).await;
                                                                    nav(&format!("/tags/{target}"), Default::default());
                                                                });
                                                            }
                                                        }
                                                    >
                                                        <option value="" selected=true>"\u{2014} Wybierz tag docelowy \u{2014}"</option>
                                                        {available.into_iter().map(|(id, name)| {
                                                            view! { <option value=id>{name}</option> }
                                                        }).collect_view()}
                                                    </select>
                                                    <button class="btn btn-ghost btn-xs" on:click=move |_| active_action.set(None)>"\u{2715}"</button>
                                                </div>
                                            }.into_any()
                                        }
                                        None => view! {}.into_any(),
                                    }
                                }}

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

                                // Subtree section
                                {if !subtree.is_empty() {
                                    view! {
                                        <div class="mb-6">
                                            <h3 class="text-xs text-base-content/50 uppercase tracking-wider mb-2">"Poddrzewo"</h3>
                                            {subtree.into_iter().map(|node| {
                                                view! {
                                                    <TagTreeRow
                                                        node=node
                                                        depth=0
                                                        tags=all_tags
                                                        new_color=new_color
                                                    />
                                                }
                                            }).collect_view()}
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {}.into_any()
                                }}

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
                            </div>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
    .into_any()
}

