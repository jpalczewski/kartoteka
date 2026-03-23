use kartoteka_shared::{CreateListRequest, List, ListTagLink, ListType, Tag};
use leptos::prelude::*;

use crate::api;
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

    let (new_name, set_new_name) = signal(String::new());
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

    let on_create = move |_| {
        let name = new_name.get();
        if name.trim().is_empty() {
            return;
        }
        let list_type = new_list_type.get();
        set_new_name.set(String::new());
        leptos::task::spawn_local(async move {
            let req = CreateListRequest { name, list_type };
            let _ = api::create_list(&req).await;
            set_refresh.update(|n| *n += 1);
        });
    };

    view! {
        <div>
            <h2 style="margin: 1rem 0;">"Twoje listy"</h2>

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
            <div class="input-row">
                <input
                    type="text"
                    placeholder="Nazwa nowej listy..."
                    prop:value=new_name
                    on:input=move |ev| set_new_name.set(event_target_value(&ev))
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        if ev.key() == "Enter" {
                            let name = new_name.get();
                            if name.trim().is_empty() { return; }
                            let list_type = new_list_type.get();
                            set_new_name.set(String::new());
                            leptos::task::spawn_local(async move {
                                let req = CreateListRequest { name, list_type };
                                let _ = api::create_list(&req).await;
                                set_refresh.update(|n| *n += 1);
                            });
                        }
                    }
                />
                <select on:change=move |ev| set_new_list_type.set(parse_list_type(&event_target_value(&ev)))>
                    <option value="custom">"Lista"</option>
                    <option value="shopping">"Zakupy"</option>
                    <option value="packing">"Pakowanie"</option>
                    <option value="project">"Projekt"</option>
                </select>
                <button class="btn" on:click=on_create>"Dodaj"</button>
            </div>

            // Lists grid
            <Suspense fallback=|| view! { <p>"Wczytywanie..."</p> }>
                {move || {
                    let lists_data = lists.get();
                    let tags_data = tags_res.get();
                    let links_data = links_res.get();

                    lists_data.map(|lists_result| {
                        match &*lists_result {
                            Err(e) => view! { <p style="color: red;">{format!("Błąd: {e}")}</p> }.into_any(),
                            Ok(all_lists) if all_lists.is_empty() => {
                                view! { <div class="empty-state">"Brak list. Utwórz pierwszą!"</div> }.into_any()
                            }
                            Ok(all_lists) => {
                                let all_tags: Vec<Tag> = tags_data
                                    .as_ref()
                                    .and_then(|r| r.as_deref().ok())
                                    .map(|s| s.to_vec())
                                    .unwrap_or_default();
                                let all_links: Vec<ListTagLink> = links_data
                                    .as_ref()
                                    .and_then(|r| r.as_deref().ok())
                                    .map(|s| s.to_vec())
                                    .unwrap_or_default();

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
                                    <div class="list-grid">
                                        {filtered_lists.into_iter().map(|list| {
                                            let list_tags: Vec<Tag> = all_tags
                                                .iter()
                                                .filter(|t| {
                                                    all_links
                                                        .iter()
                                                        .any(|link| link.list_id == list.id && link.tag_id == t.id)
                                                })
                                                .cloned()
                                                .collect();
                                            view! { <ListCard list tags=list_tags/> }
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
