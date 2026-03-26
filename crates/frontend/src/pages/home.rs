use kartoteka_shared::{CreateListRequest, List, ListTagLink, ListType, Tag};
use leptos::prelude::*;

use crate::api;
use crate::app::{ToastContext, ToastKind};
use crate::components::add_input::AddInput;
use crate::components::confirm_delete_modal::ConfirmDeleteModal;
use crate::components::list_card::{ListCard, list_type_icon, list_type_label};

#[component]
pub fn HomePage() -> impl IntoView {
    // Redirect to login if no Hanko token
    if !api::is_logged_in() {
        if let Some(w) = web_sys::window() {
            let _ = w.location().set_href("/login");
        }
    }

    let toast = use_context::<ToastContext>().expect("ToastContext missing");

    let (new_list_type, set_new_list_type) = signal(ListType::Custom);
    let (has_quantity, set_has_quantity) = signal(false);
    let (has_due_date, set_has_due_date) = signal(false);
    let (refresh, set_refresh) = signal(0u32);
    let (active_tag_filter, set_active_tag_filter) = signal(Option::<String>::None);

    // pending_delete: (list_id, list_name) — drives the modal
    let pending_delete = RwSignal::new(Option::<(String, String)>::None);

    // Lists: fetched via LocalResource, kept in writable RwSignal for optimistic updates
    let lists_res = LocalResource::new(move || {
        let _ = refresh.get();
        api::fetch_lists()
    });
    let lists_data = RwSignal::new(Vec::<List>::new());
    Effect::new(move |_| {
        if let Some(data) = lists_res.get() {
            if let Ok(lists) = data.as_deref() {
                lists_data.set(lists.to_vec());
            }
        }
    });

    // Archived lists
    let archived_res = LocalResource::new(move || {
        let _ = refresh.get();
        api::fetch_archived_lists()
    });
    let archived_data = RwSignal::new(Vec::<List>::new());
    Effect::new(move |_| {
        if let Some(data) = archived_res.get() {
            if let Ok(lists) = data.as_deref() {
                archived_data.set(lists.to_vec());
            }
        }
    });

    let tags_res = LocalResource::new(api::fetch_tags);
    let links_res = LocalResource::new(move || {
        let _ = refresh.get();
        api::fetch_list_tag_links()
    });

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
            list_tag_links
                .update(|links| links.retain(|l| !(l.list_id == list_id && l.tag_id == tag_id)));
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
        let hq = has_quantity.get();
        let hd = has_due_date.get();
        leptos::task::spawn_local(async move {
            let req = CreateListRequest {
                name,
                list_type,
                has_quantity: hq,
                has_due_date: hd,
            };
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

            // Create form — preset picker
            <div class="mb-4">
                <div class="flex flex-wrap gap-2 mb-2">
                    {[ListType::Checklist, ListType::Zakupy, ListType::Pakowanie, ListType::Terminarz, ListType::Custom]
                        .into_iter()
                        .map(|lt| {
                            let lt_for_class = lt.clone();
                            let lt_for_click = lt.clone();
                            let icon = list_type_icon(&lt);
                            let label = list_type_label(&lt);
                            view! {
                                <button
                                    type="button"
                                    class=move || {
                                        if new_list_type.get() == lt_for_class {
                                            "btn btn-sm btn-primary"
                                        } else {
                                            "btn btn-sm btn-outline"
                                        }
                                    }
                                    on:click=move |_| {
                                        set_new_list_type.set(lt_for_click.clone());
                                        match &lt_for_click {
                                            ListType::Checklist | ListType::Custom => {
                                                set_has_quantity.set(false);
                                                set_has_due_date.set(false);
                                            }
                                            ListType::Zakupy | ListType::Pakowanie => {
                                                set_has_quantity.set(true);
                                                set_has_due_date.set(false);
                                            }
                                            ListType::Terminarz => {
                                                set_has_quantity.set(false);
                                                set_has_due_date.set(true);
                                            }
                                        }
                                    }
                                >
                                    {icon} " " {label}
                                </button>
                            }
                        })
                        .collect::<Vec<_>>()}
                </div>
                <div class="flex items-center gap-4 mb-2">
                    <label class="label cursor-pointer gap-2">
                        <input
                            type="checkbox"
                            class="checkbox checkbox-sm"
                            prop:checked=has_quantity
                            on:change=move |ev| set_has_quantity.set(event_target_checked(&ev))
                        />
                        <span class="label-text">"Ilości"</span>
                    </label>
                    <label class="label cursor-pointer gap-2">
                        <input
                            type="checkbox"
                            class="checkbox checkbox-sm"
                            prop:checked=has_due_date
                            on:change=move |ev| set_has_due_date.set(event_target_checked(&ev))
                        />
                        <span class="label-text">"Terminy"</span>
                    </label>
                </div>
                <div class="flex gap-2">
                    <AddInput placeholder="Nazwa nowej listy..." button_label="Dodaj" on_submit=on_create />
                </div>
            </div>

            // Delete confirmation modal (conditionally rendered)
            {move || pending_delete.get().map(|(lid, lname)| {
                let lid_confirm = lid.clone();
                view! {
                    <ConfirmDeleteModal
                        list_id=lid
                        list_name=lname
                        on_confirm=Callback::new(move |_| {
                            let lid = lid_confirm.clone();
                            leptos::task::spawn_local(async move {
                                // Optimistic: remove from local signal
                                let removed_idx = lists_data.read().iter().position(|l| l.id == lid);
                                let removed = lists_data.read().iter().find(|l| l.id == lid).cloned();
                                lists_data.update(|ls| ls.retain(|l| l.id != lid));
                                pending_delete.set(None);

                                match api::delete_list(&lid).await {
                                    Ok(()) => toast.push("Lista usunięta".into(), ToastKind::Success),
                                    Err(e) => {
                                        // Rollback at original position
                                        if let (Some(list), Some(idx)) = (removed, removed_idx) {
                                            lists_data.update(|ls| {
                                                let idx = idx.min(ls.len());
                                                ls.insert(idx, list);
                                            });
                                        }
                                        toast.push(e, ToastKind::Error);
                                    }
                                }
                            });
                        })
                        on_cancel=Callback::new(move |_| pending_delete.set(None))
                    />
                }
            })}

            // Lists grid
            {move || {
                let tags_data = tags_res.get();
                let all_tags: Vec<Tag> = tags_data
                    .as_ref()
                    .and_then(|r| r.as_deref().ok())
                    .map(|s| s.to_vec())
                    .unwrap_or_default();
                let all_links = list_tag_links.get();
                let filter = active_tag_filter.get();

                // Show loading while resource hasn't resolved yet
                if lists_res.get().is_none() {
                    return view! {
                        <p>"Wczytywanie..."</p>
                    }.into_any();
                }

                let all_lists = lists_data.get();
                if all_lists.is_empty() {
                    return view! {
                        <div class="text-center text-base-content/50 py-12">"Brak list. Utwórz pierwszą!"</div>
                    }.into_any();
                }

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
                            let list_name = list.name.clone();
                            let list_tag_ids: Vec<String> = all_links
                                .iter()
                                .filter(|l| l.list_id == list.id)
                                .map(|l| l.tag_id.clone())
                                .collect();
                            let tog = on_list_tag_toggle.clone();
                            let tag_cb = Callback::new(move |tag_id: String| {
                                tog.run((list_id.clone(), tag_id));
                            });
                            let lid = list.id.clone();
                            view! {
                                <ListCard
                                    list
                                    all_tags=all_tags.clone()
                                    list_tag_ids
                                    on_tag_toggle=tag_cb
                                    on_delete=Callback::new(move |_: String| {
                                        pending_delete.set(Some((lid.clone(), list_name.clone())));
                                    })
                                />
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            }}

            // Archived lists section
            {move || {
                let archived = archived_data.get();
                if archived.is_empty() {
                    view! {}.into_any()
                } else {
                    view! {
                        <div class="collapse collapse-arrow bg-base-200 mt-6">
                            <input type="checkbox" />
                            <div class="collapse-title font-semibold">
                                {format!("\u{1F4E6} Archiwum ({})", archived.len())}
                            </div>
                            <div class="collapse-content">
                                <div class="flex flex-col gap-2 pt-2">
                                    {archived.into_iter().map(|list| {
                                        let lid = list.id.clone();
                                        let icon = list_type_icon(&list.list_type);
                                        view! {
                                            <div class="flex items-center justify-between p-3 bg-base-100 rounded-lg">
                                                <span class="text-base-content/70">
                                                    {icon} " " {list.name.clone()}
                                                </span>
                                                <button
                                                    type="button"
                                                    class="btn btn-ghost btn-sm"
                                                    on:click=move |_| {
                                                        let lid = lid.clone();
                                                        leptos::task::spawn_local(async move {
                                                            match api::archive_list(&lid).await {
                                                                Ok(_) => {
                                                                    archived_data.update(|ls| ls.retain(|l| l.id != lid));
                                                                    set_refresh.update(|n| *n += 1);
                                                                    toast.push("Lista przywrócona".into(), ToastKind::Success);
                                                                }
                                                                Err(e) => toast.push(e, ToastKind::Error),
                                                            }
                                                        });
                                                    }
                                                >
                                                    "\u{21A9}\u{FE0F} Przywróć"
                                                </button>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
