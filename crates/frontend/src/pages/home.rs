use kartoteka_shared::{CreateListRequest, List, ListTagLink, ListType, Tag};
use leptos::prelude::*;

use crate::api;
use crate::components::add_input::AddInput;
use crate::components::list_card::ListCard;

fn parse_list_type(s: &str) -> ListType {
    match s {
        "shopping" => ListType::Shopping,
        "packing" => ListType::Packing,
        "project" => ListType::Project,
        _ => ListType::Custom,
    }
}

#[component]
pub fn HomePage() -> impl IntoView {
    // Redirect to login if no Hanko token
    if !api::is_logged_in() {
        if let Some(w) = web_sys::window() {
            let _ = w.location().set_href("/login");
        }
    }

    let (new_list_type, set_new_list_type) = signal(ListType::Custom);
    let (refresh, set_refresh) = signal(0u32);
    let (active_tag_filter, set_active_tag_filter) = signal(Option::<String>::None);

    let lists = LocalResource::new(move || {
        let _ = refresh.get();
        api::fetch_lists()
    });

    let tags_res = LocalResource::new(|| api::fetch_tags());
    let links_res = LocalResource::new(move || {
        let _ = refresh.get();
        api::fetch_list_tag_links()
    });

    // RwSignal for optimistic tag updates, synced from LocalResource
    let list_tag_links = RwSignal::new(Vec::<ListTagLink>::new());
    Effect::new(move |_| {
        if let Some(data) = links_res.get() {
            if let Some(links) = data.as_deref().ok().map(|s| s.to_vec()) {
                list_tag_links.set(links);
            }
        }
    });

    let on_list_tag_toggle = Callback::new(move |(list_id, tag_id): (String, String)| {
        let has_tag = list_tag_links
            .read()
            .iter()
            .any(|l| l.list_id == list_id && l.tag_id == tag_id);
        if has_tag {
            list_tag_links.update(|links| {
                links.retain(|l| !(l.list_id == list_id && l.tag_id == tag_id))
            });
            let lid = list_id.clone();
            let tid = tag_id.clone();
            leptos::task::spawn_local(async move {
                let _ = api::remove_tag_from_list(&lid, &tid).await;
            });
        } else {
            list_tag_links.update(|links| {
                links.push(ListTagLink {
                    list_id: list_id.clone(),
                    tag_id: tag_id.clone(),
                })
            });
            let lid = list_id.clone();
            let tid = tag_id.clone();
            leptos::task::spawn_local(async move {
                let _ = api::assign_tag_to_list(&lid, &tid).await;
            });
        }
    });

    let on_create = Callback::new(move |name: String| {
        let list_type = new_list_type.get();
        leptos::task::spawn_local(async move {
            let req = CreateListRequest { name, list_type };
            let _ = api::create_list(&req).await;
            set_refresh.update(|n| *n += 1);
        });
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-4">"Twoje listy"</h2>

            // Tag filter bar
            <Suspense fallback=|| view! {}>
                {move || tags_res.get().map(|result| {
                    match &*result {
                        Ok(tags) if !tags.is_empty() => {
                            let tags = tags.clone();
                            view! {
                                <div class="tag-filter-bar">
                                    {tags.into_iter().map(|tag| {
                                        let tid = tag.id.clone();
                                        let tid2 = tag.id.clone();
                                        let tid3 = tag.id.clone();
                                        let color = tag.color.clone();
                                        let name = tag.name.clone();
                                        view! {
                                            <span
                                                class=move || if active_tag_filter.get().as_deref() == Some(tid.as_str()) { "tag-badge active" } else { "tag-badge" }
                                                style=format!("background: {}; color: white; cursor: pointer;", color)
                                                on:click=move |_| {
                                                    if active_tag_filter.get().as_deref() == Some(tid2.as_str()) {
                                                        set_active_tag_filter.set(None);
                                                    } else {
                                                        set_active_tag_filter.set(Some(tid3.clone()));
                                                    }
                                                }
                                            >
                                                {name}
                                            </span>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any()
                        }
                        _ => view! {}.into_any()
                    }
                })}
            </Suspense>

            // Create form
            <div class="flex gap-2 mb-4">
                <select class="select select-bordered" on:change=move |ev| set_new_list_type.set(parse_list_type(&event_target_value(&ev)))>
                    <option value="custom">"Lista"</option>
                    <option value="shopping">"Zakupy"</option>
                    <option value="packing">"Pakowanie"</option>
                    <option value="project">"Projekt"</option>
                </select>
                <AddInput placeholder="Nazwa nowej listy..." button_label="Dodaj" on_submit=on_create />
            </div>

            // Lists grid
            <Suspense fallback=|| view! { <p>"Wczytywanie..."</p> }>
                {move || {
                    let lists_data = lists.get();
                    let tags_data = tags_res.get();

                    lists_data.map(|lists_result| {
                        match &*lists_result {
                            Err(e) => view! { <p style="color: red;">{format!("Błąd: {e}")}</p> }.into_any(),
                            Ok(all_lists) if all_lists.is_empty() => {
                                view! { <div class="text-center text-base-content/50 py-12">"Brak list. Utwórz pierwszą!"</div> }.into_any()
                            }
                            Ok(all_lists) => {
                                let all_tags: Vec<Tag> = tags_data
                                    .as_ref()
                                    .and_then(|r| r.as_deref().ok())
                                    .map(|s| s.to_vec())
                                    .unwrap_or_default();
                                let all_links = list_tag_links.get();

                                let filter = active_tag_filter.get();
                                let filtered_lists: Vec<List> = all_lists
                                    .iter()
                                    .filter(|l| match &filter {
                                        None => true,
                                        Some(tag_id) => all_links
                                            .iter()
                                            .any(|link| link.list_id == l.id && &link.tag_id == tag_id),
                                    })
                                    .cloned()
                                    .collect();

                                view! {
                                    <div class="flex flex-col gap-3">
                                        {filtered_lists.into_iter().map(|list| {
                                            let list_id = list.id.clone();
                                            let list_tag_ids: Vec<String> = all_links
                                                .iter()
                                                .filter(|l| l.list_id == list.id)
                                                .map(|l| l.tag_id.clone())
                                                .collect();
                                            let tog = on_list_tag_toggle.clone();
                                            let tag_cb = Callback::new(move |tag_id: String| {
                                                tog.run((list_id.clone(), tag_id));
                                            });
                                            view! {
                                                <ListCard
                                                    list
                                                    all_tags=all_tags.clone()
                                                    list_tag_ids
                                                    on_tag_toggle=tag_cb
                                                />
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                }.into_any()
                            }
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}
