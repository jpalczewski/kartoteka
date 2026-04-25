use std::collections::BTreeMap;

use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map};

use kartoteka_shared::types::DateItem;

use crate::app::{ToastContext, ToastKind};
use crate::components::common::editable_text::EditableText;
use crate::components::common::loading::LoadingSpinner;
use crate::components::tags::color_picker::TagColorPicker;
use crate::components::tags::tag_tree::{
    TagTreeRow, build_breadcrumb, build_subtree, get_descendant_ids,
};
use crate::server_fns::tags::{
    delete_tag, get_all_tags, get_tag_items, merge_tags, update_tag, update_tag_color,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum DetailAction {
    Move,
    Merge,
}

#[component]
pub fn TagDetailPage() -> impl IntoView {
    let params = use_params_map();
    let tag_id_fn = Memo::new(move |_| params.read().get("id").unwrap_or_default());
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let navigate = StoredValue::new_local(use_navigate());

    let (refresh, set_refresh) = signal(0u32);
    let (recursive, set_recursive) = signal(true);
    let active_action = RwSignal::new(Option::<DetailAction>::None);

    let all_tags_res = Resource::new(move || refresh.get(), |_| get_all_tags());
    let items_res = Resource::new(
        move || (tag_id_fn.get(), recursive.get(), refresh.get()),
        |(id, rec, _)| get_tag_items(id, rec),
    );

    // Rename / color / delete / move / merge callbacks — each captures the current tag_id via
    // the Memo so they stay valid even if the route param changes.
    let on_rename = Callback::new(move |new_name: String| {
        if new_name.trim().is_empty() {
            return;
        }
        let id = tag_id_fn.get_untracked();
        leptos::task::spawn_local(async move {
            match update_tag(id, Some(new_name), None, None, false).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_color_change = Callback::new(move |new_color: String| {
        let id = tag_id_fn.get_untracked();
        leptos::task::spawn_local(async move {
            match update_tag_color(id, new_color).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_delete_click = move |_: leptos::ev::MouseEvent| {
        let id = tag_id_fn.get_untracked();
        leptos::task::spawn_local(async move {
            match delete_tag(id).await {
                Ok(_) => navigate.with_value(|nav| nav("/tags", Default::default())),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    let on_move_change = move |ev: leptos::ev::Event| {
        let value = event_target_value(&ev);
        let id = tag_id_fn.get_untracked();
        let clear = value.is_empty();
        let new_parent = if clear { None } else { Some(value) };
        leptos::task::spawn_local(async move {
            match update_tag(id, None, None, new_parent, clear).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
        active_action.set(None);
    };

    let on_merge_change = move |ev: leptos::ev::Event| {
        let value = event_target_value(&ev);
        if value.is_empty() {
            return;
        }
        let source = tag_id_fn.get_untracked();
        let target = value;
        let target_for_nav = target.clone();
        leptos::task::spawn_local(async move {
            match merge_tags(source, target).await {
                Ok(_) => navigate
                    .with_value(|nav| nav(&format!("/tags/{target_for_nav}"), Default::default())),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <A
                href="/tags"
                attr:class="text-sm text-base-content/50 hover:text-primary mb-2 inline-block"
            >"← Tagi"</A>

            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || {
                    let Some(tags_result) = all_tags_res.get() else { return view! {}.into_any(); };
                    let all_tags = match tags_result {
                        Ok(ts) => ts,
                        Err(e) => return view! { <p class="text-error">"Błąd: " {e.to_string()}</p> }.into_any(),
                    };
                    let tid = tag_id_fn.get();
                    let Some(current) = all_tags.iter().find(|t| t.id == tid).cloned() else {
                        return view! { <p>"Nie znaleziono tagu."</p> }.into_any();
                    };

                    let color = current.color.clone().unwrap_or_else(|| "#6366f1".to_string());
                    let tag_name = current.name.clone();
                    let tag_type = current.tag_type.clone();
                    let tag_icon = current.icon.clone();
                    let tag_id_str = current.id.clone();

                    let breadcrumb = build_breadcrumb(&all_tags, &tag_id_str);
                    let subtree = build_subtree(&all_tags, &tag_id_str);
                    let descendant_ids = get_descendant_ids(&all_tags, &tag_id_str);

                    let move_targets: Vec<(String, String)> = all_tags
                        .iter()
                        .filter(|t| t.id != tag_id_str && !descendant_ids.contains(&t.id))
                        .map(|t| (t.id.clone(), t.name.clone()))
                        .collect();
                    let merge_targets: Vec<(String, String)> = all_tags
                        .iter()
                        .filter(|t| t.id != tag_id_str)
                        .map(|t| (t.id.clone(), t.name.clone()))
                        .collect();

                    view! {
                        <div>
                            {(breadcrumb.len() > 1).then(|| view! {
                                <div class="text-sm text-base-content/50 mb-2 flex items-center gap-1 flex-wrap">
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
                                                    " › "
                                                </span>
                                            }.into_any()
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            })}

                            <div class="flex items-center gap-3 mb-4">
                                <TagColorPicker
                                    value=Signal::derive(move || color.clone())
                                    size_class="w-8 h-8"
                                    on_change=on_color_change
                                />
                                <div class="flex-1">
                                    <EditableText
                                        value=tag_name
                                        on_save=on_rename
                                        testid="tag-detail-name"
                                        class="text-2xl font-bold cursor-pointer hover:underline decoration-dotted"
                                    />
                                    <div class="text-xs text-base-content/50">{tag_type}</div>
                                </div>
                                {tag_icon.map(|i| view! { <span class="text-2xl">{i}</span> })}
                            </div>

                            <div class="flex gap-1 mb-4">
                                <button
                                    type="button"
                                    class="btn btn-ghost btn-xs"
                                    data-testid="tag-move-btn"
                                    on:click=move |_| active_action.update(|a| {
                                        *a = if *a == Some(DetailAction::Move) { None } else { Some(DetailAction::Move) };
                                    })
                                >"Przenieś"</button>
                                <button
                                    type="button"
                                    class="btn btn-ghost btn-xs"
                                    data-testid="tag-merge-btn"
                                    on:click=move |_| active_action.update(|a| {
                                        *a = if *a == Some(DetailAction::Merge) { None } else { Some(DetailAction::Merge) };
                                    })
                                >"Scal"</button>
                                <button
                                    type="button"
                                    class="btn btn-error btn-xs"
                                    data-testid="tag-delete-btn"
                                    on:click=on_delete_click
                                >"Usuń"</button>
                            </div>

                            {move || match active_action.get() {
                                Some(DetailAction::Move) => view! {
                                    <div class="flex gap-2 items-center mb-4">
                                        <select
                                            class="select select-bordered select-sm"
                                            data-testid="tag-move-select"
                                            on:change=on_move_change
                                        >
                                            <option value="" selected=true>"(brak rodzica — root)"</option>
                                            {move_targets.clone().into_iter().map(|(id, name)| {
                                                view! { <option value=id>{name}</option> }
                                            }).collect::<Vec<_>>()}
                                        </select>
                                        <button
                                            type="button"
                                            class="btn btn-ghost btn-xs"
                                            on:click=move |_| active_action.set(None)
                                        >"✕"</button>
                                    </div>
                                }.into_any(),
                                Some(DetailAction::Merge) => view! {
                                    <div class="flex gap-2 items-center mb-4">
                                        <select
                                            class="select select-bordered select-sm"
                                            data-testid="tag-merge-select"
                                            on:change=on_merge_change
                                        >
                                            <option value="" selected=true>"(wybierz tag docelowy)"</option>
                                            {merge_targets.clone().into_iter().map(|(id, name)| {
                                                view! { <option value=id>{name}</option> }
                                            }).collect::<Vec<_>>()}
                                        </select>
                                        <button
                                            type="button"
                                            class="btn btn-ghost btn-xs"
                                            on:click=move |_| active_action.set(None)
                                        >"✕"</button>
                                    </div>
                                }.into_any(),
                                None => view! {}.into_any(),
                            }}

                            {(!subtree.is_empty()).then(|| view! {
                                <div class="mb-6">
                                    <h3 class="text-xs text-base-content/50 uppercase tracking-wider mb-2">"Podtagi"</h3>
                                    {subtree.into_iter().map(|node| view! {
                                        <TagTreeRow
                                            node=node
                                            depth=0
                                            show_add_child=false
                                            show_delete=false
                                        />
                                    }).collect::<Vec<_>>()}
                                </div>
                            })}

                            <label class="flex items-center gap-2 cursor-pointer mb-4">
                                <input
                                    type="checkbox"
                                    class="toggle toggle-sm toggle-primary"
                                    data-testid="tag-recursive-toggle"
                                    prop:checked=move || recursive.get()
                                    on:change=move |ev| set_recursive.set(event_target_checked(&ev))
                                />
                                <span class="text-sm">"Uwzględnij podtagi"</span>
                            </label>

                            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                                {move || items_res.get().map(|result| match result {
                                    Err(e) => view! { <p class="text-error">"Błąd: " {e.to_string()}</p> }.into_any(),
                                    Ok(items) => render_items_grouped(items),
                                })}
                            </Suspense>
                        </div>
                    }.into_any()
                }}
            </Suspense>
        </div>
    }
}

fn render_items_grouped(items: Vec<DateItem>) -> leptos::prelude::AnyView {
    if items.is_empty() {
        return view! {
            <p class="text-center text-base-content/50 py-12" data-testid="tag-no-items">
                "Brak elementów z tym tagiem."
            </p>
        }
        .into_any();
    }

    let mut groups: BTreeMap<(String, String), Vec<DateItem>> = BTreeMap::new();
    for di in items {
        let key = (di.item.list_id.clone(), di.list_name.clone());
        groups.entry(key).or_default().push(di);
    }

    view! {
        <div data-testid="tag-items">
            {groups.into_iter().map(|((list_id, list_name), group)| {
                view! {
                    <div class="mb-6">
                        <h4 class="text-sm font-semibold uppercase tracking-wide mb-2 text-base-content/70">
                            <A href=format!("/lists/{list_id}") attr:class="link link-hover">
                                {list_name}
                            </A>
                        </h4>
                        {group.into_iter().map(|di| {
                            let completed = di.item.completed;
                            let row_class = if completed {
                                "flex items-center gap-2 py-1 pl-2 text-base-content/40"
                            } else {
                                "flex items-center gap-2 py-1 pl-2"
                            };
                            let title_class = if completed { "line-through" } else { "" };
                            view! {
                                <div class=row_class>
                                    <span>{if completed { "☑" } else { "☐" }}</span>
                                    <span class=title_class>{di.item.title.clone()}</span>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
    .into_any()
}
