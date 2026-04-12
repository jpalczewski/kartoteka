use crate::api;
use crate::api::client::GlooClient;
use crate::components::common::loading::LoadingSpinner;
use crate::components::editable_color::EditableColor;
use crate::components::editable_title::EditableTitle;
use crate::components::items::item_date_summary::ItemDateSummary;
use crate::components::tag_tree::{
    TagTreeRow, build_breadcrumb, build_subtree, get_descendant_ids,
};
use kartoteka_shared::{DateItem, Item, SearchEntityResult, Tag, UpdateTagRequest};
use leptos::prelude::*;
use leptos_fluent::move_tr;
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
    let params = use_params_map();
    let tag_id = move || params.read().get("id").unwrap_or_default();
    let navigate = use_navigate();
    let client = use_context::<GlooClient>().expect("GlooClient not provided");

    let all_tags = RwSignal::new(Vec::<Tag>::new());
    let tag = RwSignal::new(Option::<Tag>::None);
    let items = RwSignal::new(Vec::<DateItem>::new());
    let lists = RwSignal::new(Vec::<SearchEntityResult>::new());
    let (loading, set_loading) = signal(true);
    let (recursive, set_recursive) = signal(true);
    let active_action = RwSignal::new(Option::<DetailAction>::None);
    let (new_color, _set_new_color) = signal("#e94560".to_string());

    let _resource = {
        let client = client.clone();
        LocalResource::new(move || {
            let tid = tag_id();
            let rec = recursive.get();
            let client = client.clone();
            async move {
                if let Ok(tags) = api::fetch_tags(&client).await {
                    tag.set(tags.iter().find(|t| t.id == tid).cloned());
                    all_tags.set(tags);
                }
                if let Ok(fetched) = api::fetch_tag_items(&client, &tid, rec).await {
                    items.set(fetched);
                }
                if let Ok(fetched) = api::fetch_tag_entities(&client, &tid, rec, Some("list")).await
                {
                    lists.set(fetched);
                }
                set_loading.set(false);
            }
        })
    };

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            // Back button
            <A href="/tags" attr:class="text-sm text-base-content/50 hover:text-primary mb-2 inline-block">
                {move_tr!("tags-back")}
            </A>

            {
                let navigate = navigate.clone();
                move || {
                if loading.get() {
                    return view! { <LoadingSpinner/> }.into_any();
                }
                let navigate = navigate.clone();
                let client = use_context::<GlooClient>().expect("GlooClient not provided");
                match tag.get() {
                    None => view! { <p>{move_tr!("tags-not-found")}</p> }.into_any(),
                    Some(t) => {
                        let color = t.color.clone();
                        let tags_for_breadcrumb = all_tags.get();
                        let breadcrumb = build_breadcrumb(&tags_for_breadcrumb, &t.id);
                        let all_items = items.get();
                        let linked_lists = lists.get();

                        // Group items by list_name
                        let no_list_label = move_tr!("tags-no-list").get();
                        let mut groups: BTreeMap<(String, String), Vec<DateItem>> = BTreeMap::new();
                        for item in all_items {
                            let list_name = if item.list_name.is_empty() {
                                no_list_label.clone()
                            } else {
                                item.list_name.clone()
                            };
                            let list_id = item.list_id.clone();
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
                                        let client_color = client.clone();
                                        view! {
                                            <EditableColor
                                                color=color.clone()
                                                on_save=Callback::new(move |new_color: String| {
                                                    tag.update(|t| {
                                                        if let Some(t) = t { t.color = new_color.clone(); }
                                                    });
                                                    let tid = tag_id_color.clone();
                                                    let client = client_color.clone();
                                                    leptos::task::spawn_local(async move {
                                                        let req = UpdateTagRequest {
                                                            name: None,
                                                            color: Some(new_color),
                                                            parent_tag_id: None,
                                                        };
                                                        let _ = api::update_tag(&client, &tid, &req).await;
                                                    });
                                                })
                                            />
                                        }
                                    }
                                    {
                                        let tag_id_name = t.id.clone();
                                        let client_name = client.clone();
                                        view! {
                                            <EditableTitle
                                                value=t.name.clone()
                                                on_save=Callback::new(move |new_name: String| {
                                                    tag.update(|t| {
                                                        if let Some(t) = t { t.name = new_name.clone(); }
                                                    });
                                                    let tid = tag_id_name.clone();
                                                    let client = client_name.clone();
                                                    leptos::task::spawn_local(async move {
                                                        let req = UpdateTagRequest {
                                                            name: Some(new_name),
                                                            color: None,
                                                            parent_tag_id: None,
                                                        };
                                                        let _ = api::update_tag(&client, &tid, &req).await;
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
                                    >{move_tr!("tags-move-button")}</button>
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
                                    >{move_tr!("tags-merge-button")}</button>
                                    <button
                                        class="btn btn-error btn-xs"
                                        on:click={
                                            let nav = navigate.clone();
                                            let tid = tag_id_for_delete.clone();
                                            let client_del = client.clone();
                                            move |_| {
                                                let tid = tid.clone();
                                                let nav = nav.clone();
                                                let client = client_del.clone();
                                                leptos::task::spawn_local(async move {
                                                    let _ = api::delete_tag(&client, &tid).await;
                                                    nav("/tags", Default::default());
                                                });
                                            }
                                        }
                                    >{move_tr!("tags-delete-button")}</button>
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
                                                            let client_move = client.clone();
                                                            move |ev| {
                                                                let value = event_target_value(&ev);
                                                                let new_parent = if value.is_empty() { None } else { Some(value) };
                                                                let tid = tid.clone();
                                                                let client = client_move.clone();
                                                                leptos::task::spawn_local(async move {
                                                                    let req = UpdateTagRequest {
                                                                        name: None,
                                                                        color: None,
                                                                        parent_tag_id: Some(new_parent),
                                                                    };
                                                                    let _ = api::update_tag(&client, &tid, &req).await;
                                                                    if let Ok(fetched) = api::fetch_tags(&client).await {
                                                                        all_tags.set(fetched);
                                                                    }
                                                                    active_action.set(None);
                                                                });
                                                            }
                                                        }
                                                    >
                                                        <option value="" selected=true>{move_tr!("tags-no-parent-option")}</option>
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
                                                            let client_merge = client.clone();
                                                            move |ev| {
                                                                let value = event_target_value(&ev);
                                                                if value.is_empty() { return; }
                                                                let tid = tid.clone();
                                                                let target = value.clone();
                                                                let nav = nav.clone();
                                                                let client = client_merge.clone();
                                                                leptos::task::spawn_local(async move {
                                                                    let _ = api::merge_tag(&client, &tid, &target).await;
                                                                    nav(&format!("/tags/{target}"), Default::default());
                                                                });
                                                            }
                                                        }
                                                    >
                                                        <option value="" selected=true>{move_tr!("tags-merge-select-option")}</option>
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
                                    <span class="text-sm">{move_tr!("tags-include-subtags")}</span>
                                </label>

                                // Subtree section
                                {if !subtree.is_empty() {
                                    view! {
                                        <div class="mb-6">
                                            <h3 class="text-xs text-base-content/50 uppercase tracking-wider mb-2">{move_tr!("tags-subtree-title")}</h3>
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

                                {if linked_lists.is_empty() {
                                    view! {
                                        <p class="text-center text-base-content/50 py-4">
                                            {move_tr!("tags-no-lists")}
                                        </p>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="mb-8">
                                            <h3 class="text-xs text-base-content/50 uppercase tracking-wider mb-3">{move_tr!("tags-lists-title")}</h3>
                                            <div class="flex flex-col gap-3">
                                                {linked_lists.into_iter().map(|list| {
                                                    let archived = list.archived.unwrap_or(false);
                                                    let type_label = match list.list_type.unwrap_or(kartoteka_shared::ListType::Custom) {
                                                        kartoteka_shared::ListType::Checklist => move_tr!("lists-type-checklist"),
                                                        kartoteka_shared::ListType::Zakupy => move_tr!("lists-type-shopping"),
                                                        kartoteka_shared::ListType::Pakowanie => move_tr!("lists-type-packing"),
                                                        kartoteka_shared::ListType::Terminarz => move_tr!("lists-type-schedule"),
                                                        kartoteka_shared::ListType::Custom => move_tr!("lists-type-custom"),
                                                    };
                                                    view! {
                                                        <article class="card bg-base-200 border border-base-300">
                                                            <div class="card-body gap-3">
                                                                <div class="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
                                                                    <div class="flex flex-col gap-1">
                                                                        <A href=format!("/lists/{}", list.id) attr:class="text-lg font-semibold link link-hover text-primary">
                                                                            {list.name.clone()}
                                                                        </A>
                                                                        <span class="text-sm text-base-content/60">{type_label}</span>
                                                                    </div>
                                                                    <div class="flex flex-wrap gap-2">
                                                                        {archived.then(|| view! {
                                                                            <span class="badge badge-warning">{move_tr!("search-status-archived")}</span>
                                                                        })}
                                                                    </div>
                                                                </div>
                                                                {list.description.as_ref().map(|description| view! {
                                                                    <p class="text-sm text-base-content/70 whitespace-pre-wrap">{description.clone()}</p>
                                                                })}
                                                            </div>
                                                        </article>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        </div>
                                    }.into_any()
                                }}

                                // Items grouped by list
                                {if groups.is_empty() {
                                    view! {
                                        <p class="text-center text-base-content/50 py-12">
                                            {move_tr!("tags-no-items")}
                                        </p>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div>
                                            <h3 class="text-xs text-base-content/50 uppercase tracking-wider mb-3">{move_tr!("tags-items-title")}</h3>
                                            {groups.into_iter().map(|((list_id, list_name), group_items)| {
                                                view! {
                                                    <div class="mb-6">
                                                        <h4 class="text-sm font-semibold uppercase tracking-wide mb-2 text-base-content/70">
                                                            <A href=format!("/lists/{list_id}") attr:class="link link-hover">
                                                                {list_name}
                                                            </A>
                                                        </h4>
                                                        {group_items.into_iter().map(|item| {
                                                            let detail_href = format!("/lists/{}/items/{}", item.list_id, item.id);
                                                            let completed = item.completed;
                                                            let title = item.title.clone();
                                                            let item: Item = item.into();
                                                            view! {
                                                                <A
                                                                    href=detail_href
                                                                    attr:class="block rounded-box border border-transparent px-3 py-2 transition-colors hover:border-base-300 hover:bg-base-200/60"
                                                                >
                                                                    <div class="flex items-start gap-2">
                                                                        <span class=if completed {
                                                                            "pt-0.5 text-base-content/40"
                                                                        } else {
                                                                            "pt-0.5 text-base-content/70"
                                                                        }>
                                                                            {if completed { "\u{2611}" } else { "\u{2610}" }}
                                                                        </span>
                                                                        <div class="min-w-0 flex-1">
                                                                            <div class=if completed {
                                                                                "truncate line-through text-base-content/40"
                                                                            } else {
                                                                                "truncate text-base-content"
                                                                            }>
                                                                                {title}
                                                                            </div>
                                                                            <ItemDateSummary item=item/>
                                                                        </div>
                                                                    </div>
                                                                </A>
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
