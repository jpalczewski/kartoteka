use kartoteka_shared::{
    Container, CreateContainerRequest, CreateListRequest, List, ListTagLink, Tag,
};
use leptos::prelude::*;
use leptos_fluent::move_tr;

use crate::api;
use crate::api::client::GlooClient;
use crate::app::{ToastContext, ToastKind};
use crate::components::common::loading::LoadingSpinner;
use crate::components::confirm_delete_modal::ConfirmDeleteModal;
use crate::components::container_card::ContainerCard;
use crate::components::create_entity_input::CreateEntityInput;
use crate::components::list_card::{ListCard, list_type_icon};

#[component]
pub fn HomePage() -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let client = use_context::<GlooClient>().expect("GlooClient not provided");

    let (refresh, set_refresh) = signal(0u32);
    let (active_tag_filter, set_active_tag_filter) = signal(Option::<String>::None);
    let pending_delete = RwSignal::new(Option::<(String, String)>::None);

    // Home data: pinned + recent + root
    let home_res = {
        let client = client.clone();
        LocalResource::new(move || {
            let _ = refresh.get();
            let client = client.clone();
            async move { api::fetch_home(&client).await }
        })
    };

    let pinned_lists = RwSignal::new(Vec::<List>::new());
    let pinned_containers = RwSignal::new(Vec::<Container>::new());
    let recent_lists = RwSignal::new(Vec::<List>::new());
    let recent_containers = RwSignal::new(Vec::<Container>::new());
    let root_containers = RwSignal::new(Vec::<Container>::new());
    let root_lists = RwSignal::new(Vec::<List>::new());

    Effect::new(move |_| {
        if let Some(Ok(data)) = home_res.get() {
            pinned_lists.set(data.pinned_lists);
            pinned_containers.set(data.pinned_containers);
            recent_lists.set(data.recent_lists);
            recent_containers.set(data.recent_containers);
            root_containers.set(data.root_containers);
            root_lists.set(data.root_lists);
        }
    });

    // Archived lists
    let archived_res = {
        let client = client.clone();
        LocalResource::new(move || {
            let _ = refresh.get();
            let client = client.clone();
            async move { api::fetch_archived_lists(&client).await }
        })
    };
    let archived_data = RwSignal::new(Vec::<List>::new());
    Effect::new(move |_| {
        if let Some(Ok(lists)) = archived_res.get() {
            archived_data.set(lists);
        }
    });

    let tags_res = {
        let client = client.clone();
        LocalResource::new(move || {
            let client = client.clone();
            async move { api::fetch_tags(&client).await }
        })
    };

    let links_res = {
        let client = client.clone();
        LocalResource::new(move || {
            let _ = refresh.get();
            let client = client.clone();
            async move { api::fetch_list_tag_links(&client).await }
        })
    };

    let list_tag_links = RwSignal::new(Vec::<ListTagLink>::new());
    Effect::new(move |_| {
        if let Some(Ok(links)) = links_res.get() {
            list_tag_links.set(links);
        }
    });

    let on_list_tag_toggle = {
        let client = client.clone();
        Callback::new(move |(list_id, tag_id): (String, String)| {
            let has_tag = list_tag_links
                .read()
                .iter()
                .any(|l| l.list_id == list_id && l.tag_id == tag_id);
            if has_tag {
                list_tag_links
                    .update(|links| links.retain(|l| !(l.list_id == list_id && l.tag_id == tag_id)));
                let client = client.clone();
                leptos::task::spawn_local(async move {
                    let _ = api::remove_tag_from_list(&client, &list_id, &tag_id).await;
                });
            } else {
                list_tag_links.update(|links| {
                    links.push(ListTagLink {
                        list_id: list_id.clone(),
                        tag_id: tag_id.clone(),
                    })
                });
                let client = client.clone();
                leptos::task::spawn_local(async move {
                    let _ = api::assign_tag_to_list(&client, &list_id, &tag_id).await;
                });
            }
        })
    };

    let on_create_list = {
        let client = client.clone();
        Callback::new(move |req: CreateListRequest| {
            let client = client.clone();
            leptos::task::spawn_local(async move {
                match api::create_list(&client, &req).await {
                    Ok(_) => set_refresh.update(|n| *n += 1),
                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                }
            });
        })
    };

    let on_create_container = {
        let client = client.clone();
        Callback::new(move |req: CreateContainerRequest| {
            let client = client.clone();
            leptos::task::spawn_local(async move {
                match api::create_container(&client, &req).await {
                    Ok(_) => set_refresh.update(|n| *n += 1),
                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                }
            });
        })
    };

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-4">{move_tr!("home-title")}</h2>

            // Tag filter bar
            <Suspense fallback=|| view! {}>
                {move || tags_res.get().map(|result| {
                    match result.as_ref() {
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
            <CreateEntityInput
                show_container_options=true
                on_create_list=on_create_list
                on_create_container=on_create_container
            />

            // Delete confirmation modal
            {move || pending_delete.get().map(|(lid, lname)| {
                let lid_confirm = lid.clone();
                let client = use_context::<GlooClient>().expect("GlooClient not provided");
                view! {
                    <ConfirmDeleteModal
                        list_id=lid
                        list_name=lname
                        on_confirm=Callback::new(move |_| {
                            let lid = lid_confirm.clone();
                            let client = client.clone();
                            leptos::task::spawn_local(async move {
                                let removed_idx = root_lists.read().iter().position(|l| l.id == lid);
                                let removed = root_lists.read().iter().find(|l| l.id == lid).cloned();
                                root_lists.update(|ls| ls.retain(|l| l.id != lid));
                                pinned_lists.update(|ls| ls.retain(|l| l.id != lid));
                                recent_lists.update(|ls| ls.retain(|l| l.id != lid));
                                pending_delete.set(None);

                                match api::delete_list(&client, &lid).await {
                                    Ok(()) => toast.push(move_tr!("home-list-deleted").get(), ToastKind::Success),
                                    Err(e) => {
                                        if let (Some(list), Some(idx)) = (removed, removed_idx) {
                                            root_lists.update(|ls| {
                                                let idx = idx.min(ls.len());
                                                ls.insert(idx, list);
                                            });
                                        }
                                        toast.push(e.to_string(), ToastKind::Error);
                                    }
                                }
                            });
                        })
                        on_cancel=Callback::new(move |_| pending_delete.set(None))
                    />
                }
            })}

            {move || {
                if home_res.get().is_none() {
                    return view! { <LoadingSpinner/> }.into_any();
                }

                let all_tags: Vec<Tag> = tags_res
                    .get()
                    .and_then(|r| r.ok())
                    .unwrap_or_default();
                let all_links = list_tag_links.get();
                let filter = active_tag_filter.get();

                let pl = pinned_lists.get();
                let pc = pinned_containers.get();
                let rl = recent_lists.get();
                let rc = recent_containers.get();
                let rc_list = root_containers.get();
                let rll = root_lists.get();

                let filter_list = |list: &List| match &filter {
                    None => true,
                    Some(tag_id) => all_links
                        .iter()
                        .any(|link| link.list_id == list.id && &link.tag_id == tag_id),
                };

                let client_inner = use_context::<GlooClient>().expect("GlooClient not provided");

                view! {
                    <div>
                        // === Pinned section ===
                        {if !pl.is_empty() || !pc.is_empty() {
                            let pl = pl.clone();
                            let pc = pc.clone();
                            let all_tags = all_tags.clone();
                            let all_links = all_links.clone();
                            let client_p = client_inner.clone();
                            view! {
                                <div class="collapse collapse-arrow bg-base-200 mb-4">
                                    <input type="checkbox" checked />
                                    <div class="collapse-title font-semibold">"📌 " {move_tr!("home-pinned")}</div>
                                    <div class="collapse-content">
                                        <div class="flex flex-col gap-3 pt-2">
                                            {pc.into_iter().map(|c| {
                                                let cid = c.id.clone();
                                                let client_pp = client_p.clone();
                                                view! {
                                                    <ContainerCard
                                                        container=c
                                                        on_pin=Callback::new(move |_| {
                                                            let cid = cid.clone();
                                                            let client_pp = client_pp.clone();
                                                            leptos::task::spawn_local(async move {
                                                                let _ = api::toggle_container_pin(&client_pp, &cid).await;
                                                                set_refresh.update(|n| *n += 1);
                                                            });
                                                        })
                                                    />
                                                }
                                            }).collect::<Vec<_>>()}
                                            {pl.into_iter().filter(|l| filter_list(l)).map(|list| {
                                                let list_id = list.id.clone();
                                                let list_name = list.name.clone();
                                                let lid_del = list.id.clone();
                                                let list_tag_ids: Vec<String> = all_links
                                                    .iter()
                                                    .filter(|l| l.list_id == list.id)
                                                    .map(|l| l.tag_id.clone())
                                                    .collect();
                                                let tog = on_list_tag_toggle.clone();
                                                let tag_cb = Callback::new(move |tag_id: String| tog.run((list_id.clone(), tag_id)));
                                                view! {
                                                    <ListCard
                                                        list
                                                        all_tags=all_tags.clone()
                                                        list_tag_ids
                                                        on_tag_toggle=tag_cb
                                                        on_delete=Callback::new(move |_: String| {
                                                            pending_delete.set(Some((lid_del.clone(), list_name.clone())));
                                                        })
                                                    />
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}

                        // === Recent section ===
                        {if !rl.is_empty() || !rc.is_empty() {
                            let rl = rl.clone();
                            let rc = rc.clone();
                            let all_tags = all_tags.clone();
                            let all_links = all_links.clone();
                            view! {
                                <div class="collapse collapse-arrow bg-base-200 mb-4">
                                    <input type="checkbox" checked />
                                    <div class="collapse-title font-semibold">"🕐 " {move_tr!("home-recent")}</div>
                                    <div class="collapse-content">
                                        <div class="flex flex-col gap-3 pt-2">
                                            {rc.into_iter().map(|c| {
                                                view! {
                                                    <ContainerCard container=c />
                                                }
                                            }).collect::<Vec<_>>()}
                                            {rl.into_iter().filter(|l| filter_list(l)).map(|list| {
                                                let list_id = list.id.clone();
                                                let list_name = list.name.clone();
                                                let lid_del = list.id.clone();
                                                let list_tag_ids: Vec<String> = all_links
                                                    .iter()
                                                    .filter(|l| l.list_id == list.id)
                                                    .map(|l| l.tag_id.clone())
                                                    .collect();
                                                let tog = on_list_tag_toggle.clone();
                                                let tag_cb = Callback::new(move |tag_id: String| tog.run((list_id.clone(), tag_id)));
                                                view! {
                                                    <ListCard
                                                        list
                                                        all_tags=all_tags.clone()
                                                        list_tag_ids
                                                        on_tag_toggle=tag_cb
                                                        on_delete=Callback::new(move |_: String| {
                                                            pending_delete.set(Some((lid_del.clone(), list_name.clone())));
                                                        })
                                                    />
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}

                        // === Root containers ===
                        {if !rc_list.is_empty() {
                            let rc_list = rc_list.clone();
                            let client_rc = client_inner.clone();
                            view! {
                                <div class="mb-4">
                                    <h3 class="text-sm font-semibold text-base-content/60 mb-2 uppercase tracking-wide">{move_tr!("home-folders-and-projects")}</h3>
                                    <div class="flex flex-col gap-3">
                                        {rc_list.into_iter().map(|c| {
                                            let cid = c.id.clone();
                                            let cid_del = c.id.clone();
                                            let client_del = client_rc.clone();
                                            let client_pn = client_rc.clone();
                                            view! {
                                                <ContainerCard
                                                    container=c
                                                    on_delete=Callback::new(move |_: String| {
                                                        let cid = cid_del.clone();
                                                        let client_del = client_del.clone();
                                                        leptos::task::spawn_local(async move {
                                                            match api::delete_container(&client_del, &cid).await {
                                                                Ok(()) => {
                                                                    root_containers.update(|cs| cs.retain(|c| c.id != cid));
                                                                    toast.push(move_tr!("home-container-deleted").get(), ToastKind::Success);
                                                                }
                                                                Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                                            }
                                                        });
                                                    })
                                                    on_pin=Callback::new(move |_| {
                                                        let cid = cid.clone();
                                                        let client_pn = client_pn.clone();
                                                        leptos::task::spawn_local(async move {
                                                            let _ = api::toggle_container_pin(&client_pn, &cid).await;
                                                            set_refresh.update(|n| *n += 1);
                                                        });
                                                    })
                                                />
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}

                        // === Root lists ===
                        {
                            let filtered: Vec<List> = rll.iter()
                                .filter(|l| filter_list(l))
                                .cloned()
                                .collect();

                            if filtered.is_empty() && rc_list.is_empty() && pl.is_empty() && pc.is_empty() {
                                view! {
                                    <div class="text-center text-base-content/50 py-12">{move_tr!("home-empty")}</div>
                                }.into_any()
                            } else if filtered.is_empty() {
                                view! {}.into_any()
                            } else {
                                let all_tags = all_tags.clone();
                                let all_links = all_links.clone();
                                view! {
                                    <div>
                                        {if !rc_list.is_empty() {
                                            view! {
                                                <h3 class="text-sm font-semibold text-base-content/60 mb-2 uppercase tracking-wide">{move_tr!("home-lists")}</h3>
                                            }.into_any()
                                        } else { view! {}.into_any() }}
                                        <div class="flex flex-col gap-3">
                                            {filtered.into_iter().map(|list| {
                                                let list_id = list.id.clone();
                                                let list_name = list.name.clone();
                                                let lid_del = list.id.clone();
                                                let list_tag_ids: Vec<String> = all_links
                                                    .iter()
                                                    .filter(|l| l.list_id == list.id)
                                                    .map(|l| l.tag_id.clone())
                                                    .collect();
                                                let tog = on_list_tag_toggle.clone();
                                                let tag_cb = Callback::new(move |tag_id: String| tog.run((list_id.clone(), tag_id)));
                                                view! {
                                                    <ListCard
                                                        list
                                                        all_tags=all_tags.clone()
                                                        list_tag_ids
                                                        on_tag_toggle=tag_cb
                                                        on_delete=Callback::new(move |_: String| {
                                                            pending_delete.set(Some((lid_del.clone(), list_name.clone())));
                                                        })
                                                    />
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                }.into_any()
                            }
                        }
                    </div>
                }.into_any()
            }}

            // Archived lists section
            {move || {
                let archived = archived_data.get();
                if archived.is_empty() {
                    view! {}.into_any()
                } else {
                    let archived_count = archived.len();
                    let client_arch = use_context::<GlooClient>().expect("GlooClient not provided");
                    view! {
                        <div class="collapse collapse-arrow bg-base-200 mt-6">
                            <input type="checkbox" />
                            <div class="collapse-title font-semibold">
                                "📦 " {move_tr!("home-archive", { "count" => archived_count })}
                            </div>
                            <div class="collapse-content">
                                <div class="flex flex-col gap-2 pt-2">
                                    {archived.into_iter().map(|list| {
                                        let lid = list.id.clone();
                                        let icon = list_type_icon(&list.list_type);
                                        let client_a = client_arch.clone();
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
                                                        let client_a = client_a.clone();
                                                        leptos::task::spawn_local(async move {
                                                            match api::archive_list(&client_a, &lid).await {
                                                                Ok(_) => {
                                                                    archived_data.update(|ls| ls.retain(|l| l.id != lid));
                                                                    set_refresh.update(|n| *n += 1);
                                                                    toast.push(move_tr!("home-list-restored").get(), ToastKind::Success);
                                                                }
                                                                Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                                            }
                                                        });
                                                    }
                                                >
                                                    {move_tr!("home-restore-button")}
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
