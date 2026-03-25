use crate::api;
use crate::components::add_input::AddInput;
use crate::components::tag_badge::TagBadge;
use crate::components::tag_tree::{build_tag_tree, TagNode};
use kartoteka_shared::{CreateTagRequest, Tag, UpdateTagRequest};
use leptos::prelude::*;
use leptos_router::components::A;

#[derive(Clone, PartialEq)]
enum TagAction {
    AddChild,
    Rename,
    Move,
    Merge,
}

fn get_descendant_ids(tags: &[Tag], tag_id: &str) -> Vec<String> {
    let mut descendants = Vec::new();
    let mut stack = vec![tag_id.to_string()];
    while let Some(id) = stack.pop() {
        for tag in tags {
            if tag.parent_tag_id.as_deref() == Some(&id) {
                descendants.push(tag.id.clone());
                stack.push(tag.id.clone());
            }
        }
    }
    descendants
}

#[component]
pub fn TagsPage() -> impl IntoView {
    if !api::is_logged_in() {
        return view! { <p><a href="/login">"Zaloguj się"</a></p> }.into_any();
    }

    let tags = RwSignal::new(Vec::<Tag>::new());
    let (loading, set_loading) = signal(true);
    let (new_color, set_new_color) = signal("#e94560".to_string());
    let active_action = RwSignal::new(Option::<(String, TagAction)>::None);

    // Initial fetch
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
                                    active_action=active_action
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

#[component]
fn TagTreeRow(
    node: TagNode,
    depth: usize,
    tags: RwSignal<Vec<Tag>>,
    active_action: RwSignal<Option<(String, TagAction)>>,
    new_color: ReadSignal<String>,
) -> impl IntoView {
    let tag = node.tag;
    let children = node.children;
    let tid = tag.id.clone();
    let tid_link = tag.id.clone();
    let tid_add = tag.id.clone();
    let tid_rename = tag.id.clone();
    let tid_move = tag.id.clone();
    let tid_merge = tag.id.clone();
    let tid_delete = tag.id.clone();
    let tid_action = tag.id.clone();
    let padding = format!("padding-left: {}rem;", depth as f64 * 1.0);

    view! {
        <div>
            <div class="flex items-center gap-1 py-1" style=padding.clone()>
                <A href=format!("/tags/{tid_link}") attr:class="no-underline">
                    <TagBadge tag=tag.clone() />
                </A>
                <button
                    class="btn btn-ghost btn-xs btn-square"
                    title="Dodaj podtag"
                    on:click={
                        let tid = tid_add.clone();
                        move |_| {
                            let current = active_action.get_untracked();
                            if current.as_ref() == Some(&(tid.clone(), TagAction::AddChild)) {
                                active_action.set(None);
                            } else {
                                active_action.set(Some((tid.clone(), TagAction::AddChild)));
                            }
                        }
                    }
                >"+"</button>
                <button
                    class="btn btn-ghost btn-xs btn-square"
                    title="Zmień nazwę"
                    on:click={
                        let tid = tid_rename.clone();
                        move |_| {
                            let current = active_action.get_untracked();
                            if current.as_ref() == Some(&(tid.clone(), TagAction::Rename)) {
                                active_action.set(None);
                            } else {
                                active_action.set(Some((tid.clone(), TagAction::Rename)));
                            }
                        }
                    }
                >"\u{270E}"</button>
                <button
                    class="btn btn-ghost btn-xs btn-square"
                    title="Przenieś"
                    on:click={
                        let tid = tid_move.clone();
                        move |_| {
                            let current = active_action.get_untracked();
                            if current.as_ref() == Some(&(tid.clone(), TagAction::Move)) {
                                active_action.set(None);
                            } else {
                                active_action.set(Some((tid.clone(), TagAction::Move)));
                            }
                        }
                    }
                >"\u{2195}"</button>
                <button
                    class="btn btn-ghost btn-xs btn-square"
                    title="Scal z innym"
                    on:click={
                        let tid = tid_merge.clone();
                        move |_| {
                            let current = active_action.get_untracked();
                            if current.as_ref() == Some(&(tid.clone(), TagAction::Merge)) {
                                active_action.set(None);
                            } else {
                                active_action.set(Some((tid.clone(), TagAction::Merge)));
                            }
                        }
                    }
                >"\u{2295}"</button>
                <button
                    class="btn btn-error btn-xs btn-square"
                    title="Usuń"
                    on:click={
                        let tid = tid_delete.clone();
                        move |_| {
                            tags.update(|t| t.retain(|tag| tag.id != tid));
                            let tid = tid.clone();
                            leptos::task::spawn_local(async move {
                                let _ = api::delete_tag(&tid).await;
                            });
                        }
                    }
                >"\u{2715}"</button>
            </div>

            // Inline action forms
            {
                let tid_for_action = tid_action.clone();
                move || {
                    let current = active_action.get();
                    match current {
                        Some((ref action_id, ref action)) if action_id == &tid_for_action => {
                            let child_padding = format!("padding-left: {}rem;", (depth + 1) as f64 * 1.0);
                            match action {
                                TagAction::AddChild => {
                                    let tid_create = tid_for_action.clone();
                                    let on_submit_child = Callback::new(move |name: String| {
                                        let color = new_color.get_untracked();
                                        let parent_id = tid_create.clone();
                                        leptos::task::spawn_local(async move {
                                            let req = CreateTagRequest {
                                                name,
                                                color,
                                                parent_tag_id: Some(parent_id),
                                            };
                                            if let Ok(tag) = api::create_tag(&req).await {
                                                tags.update(|t| t.push(tag));
                                            }
                                            active_action.set(None);
                                        });
                                    });
                                    view! {
                                        <div class="flex gap-2 items-center py-1" style=child_padding>
                                            <AddInput placeholder="Nazwa podtagu..." button_label="Dodaj" on_submit=on_submit_child />
                                            <button class="btn btn-ghost btn-xs" on:click=move |_| active_action.set(None)>"\u{2715}"</button>
                                        </div>
                                    }.into_any()
                                }
                                TagAction::Rename => {
                                    let tid_rename = tid_for_action.clone();
                                    let on_submit_rename = Callback::new(move |new_name: String| {
                                        let tid = tid_rename.clone();
                                        let name_clone = new_name.clone();
                                        tags.update(|t| {
                                            if let Some(tag) = t.iter_mut().find(|tag| tag.id == tid) {
                                                tag.name = name_clone;
                                            }
                                        });
                                        leptos::task::spawn_local(async move {
                                            let req = UpdateTagRequest {
                                                name: Some(new_name),
                                                color: None,
                                                parent_tag_id: None,
                                            };
                                            let _ = api::update_tag(&tid, &req).await;
                                        });
                                        active_action.set(None);
                                    });
                                    view! {
                                        <div class="flex gap-2 items-center py-1" style=child_padding>
                                            <AddInput placeholder="Nowa nazwa..." button_label="Zmień" on_submit=on_submit_rename />
                                            <button class="btn btn-ghost btn-xs" on:click=move |_| active_action.set(None)>"\u{2715}"</button>
                                        </div>
                                    }.into_any()
                                }
                                TagAction::Move => {
                                    let all_tags = tags.get_untracked();
                                    let tid_move = tid_for_action.clone();
                                    let descendant_ids = get_descendant_ids(&all_tags, &tid_move);
                                    let available: Vec<(String, String)> = all_tags
                                        .iter()
                                        .filter(|t| t.id != tid_move && !descendant_ids.contains(&t.id))
                                        .map(|t| (t.id.clone(), t.name.clone()))
                                        .collect();

                                    view! {
                                        <div class="flex gap-2 items-center py-1" style=child_padding>
                                            <select
                                                class="select select-bordered select-sm"
                                                on:change={
                                                    let tid_move = tid_move.clone();
                                                    move |ev| {
                                                        let value = event_target_value(&ev);
                                                        let new_parent = if value.is_empty() {
                                                            None
                                                        } else {
                                                            Some(value)
                                                        };
                                                        let tid = tid_move.clone();
                                                        leptos::task::spawn_local(async move {
                                                            let req = UpdateTagRequest {
                                                                name: None,
                                                                color: None,
                                                                parent_tag_id: Some(new_parent),
                                                            };
                                                            let _ = api::update_tag(&tid, &req).await;
                                                            if let Ok(fetched) = api::fetch_tags().await {
                                                                tags.set(fetched);
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
                                TagAction::Merge => {
                                    let all_tags = tags.get_untracked();
                                    let tid_merge = tid_for_action.clone();
                                    let available: Vec<(String, String)> = all_tags
                                        .iter()
                                        .filter(|t| t.id != tid_merge)
                                        .map(|t| (t.id.clone(), t.name.clone()))
                                        .collect();

                                    view! {
                                        <div class="flex gap-2 items-center py-1" style=child_padding>
                                            <select
                                                class="select select-bordered select-sm"
                                                on:change={
                                                    let tid_merge = tid_merge.clone();
                                                    move |ev| {
                                                        let value = event_target_value(&ev);
                                                        if value.is_empty() {
                                                            return;
                                                        }
                                                        let tid = tid_merge.clone();
                                                        leptos::task::spawn_local(async move {
                                                            let _ = api::merge_tag(&tid, &value).await;
                                                            if let Ok(fetched) = api::fetch_tags().await {
                                                                tags.set(fetched);
                                                            }
                                                            active_action.set(None);
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
                            }
                        }
                        _ => view! {}.into_any(),
                    }
                }
            }

            // Children
            {children.into_iter().map(|child| {
                view! {
                    <TagTreeRow
                        node=child
                        depth=depth + 1
                        tags=tags
                        active_action=active_action
                        new_color=new_color
                    />
                }
            }).collect_view()}
        </div>
    }
    .into_any()
}
