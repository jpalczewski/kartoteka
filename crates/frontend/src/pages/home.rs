use kartoteka_shared::{
    Container, CreateContainerRequest, CreateListRequest, List, ListTagLink,
    ReorderContainersRequest, SetListPlacementRequest, Tag,
};
use leptos::prelude::*;
use leptos_fluent::move_tr;

use crate::api;
use crate::api::client::GlooClient;
use crate::app::{SessionResource, ToastContext, ToastKind};
use crate::components::common::dnd::{
    DragHandleButton, DragHandleLabel, DragShell, DragSurface, ReorderDropTarget,
};
use crate::components::common::loading::LoadingSpinner;
use crate::components::confirm_delete_modal::ConfirmDeleteModal;
use crate::components::container_card::ContainerCard;
use crate::components::create_entity_input::CreateEntityInput;
use crate::components::list_card::{ListCard, list_type_icon};
use crate::components::tags::tag_filter_bar::TagFilterBar;
use crate::components::tags::tag_tree::build_tag_filter_options;
use crate::state::dnd::{DndState, DropTarget, reorder_ids_for_target};
use crate::state::item_mutations::run_optimistic_mutation;
use crate::state::reorder::apply_reorder;

#[component]
fn LandingPage() -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center min-h-[60vh] p-4">
            <div class="card bg-base-200 border border-base-300 w-full max-w-sm text-center">
                <div class="card-body items-center gap-4">
                    <h1 class="text-3xl font-bold text-primary">{move_tr!("app-title")}</h1>
                    <p class="text-base-content/70">{move_tr!("landing-tagline")}</p>
                    <div class="flex gap-3 mt-2">
                        <a href="/login" class="btn btn-primary">{move_tr!("auth-login-button")}</a>
                        <a href="/signup" class="btn btn-outline">{move_tr!("auth-signup-button")}</a>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn HomePage() -> impl IntoView {
    let session_res = use_context::<SessionResource>().expect("SessionResource missing");

    view! {
        {move || match session_res.get() {
            None => view! { <LoadingSpinner/> }.into_any(),
            Some(None) => view! { <LandingPage/> }.into_any(),
            Some(Some(_)) => view! { <HomePageInner/> }.into_any(),
        }}
    }
}

#[component]
fn HomePageInner() -> impl IntoView {
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
    let root_container_dnd_state = RwSignal::new(DndState::default());
    let root_list_dnd_state = RwSignal::new(DndState::default());

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
                list_tag_links.update(|links| {
                    links.retain(|l| !(l.list_id == list_id && l.tag_id == tag_id))
                });
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

    let on_root_container_drop = {
        let client = client.clone();
        Callback::new(move |target: DropTarget| {
            let Some(dragged_id) = root_container_dnd_state.get_untracked().dragged_id.clone()
            else {
                return;
            };
            let current_ids: Vec<String> = root_containers
                .get_untracked()
                .into_iter()
                .map(|container| container.id)
                .collect();
            let Some(next_ids) = reorder_ids_for_target(&current_ids, &dragged_id, &target) else {
                return;
            };

            let request = ReorderContainersRequest {
                container_ids: next_ids.clone(),
                parent_container_id: None,
            };
            let dragged_id_for_mutation = dragged_id.clone();
            let target_for_mutation = target.clone();
            let client = client.clone();
            run_optimistic_mutation(
                root_containers,
                move |containers| {
                    let current_ids: Vec<String> = containers
                        .iter()
                        .map(|container| container.id.clone())
                        .collect();
                    let Some(next_ids) = reorder_ids_for_target(
                        &current_ids,
                        &dragged_id_for_mutation,
                        &target_for_mutation,
                    ) else {
                        return false;
                    };
                    apply_reorder(containers, &next_ids, |container| container.id.as_str())
                },
                move || async move { api::reorder_containers(&client, &request).await },
                move |error| toast.push(error.to_string(), ToastKind::Error),
            );
        })
    };

    let on_root_list_drop = {
        let client = client.clone();
        Callback::new(move |target: DropTarget| {
            let Some(dragged_id) = root_list_dnd_state.get_untracked().dragged_id.clone() else {
                return;
            };
            let current_ids: Vec<String> = root_lists
                .get_untracked()
                .into_iter()
                .map(|list| list.id)
                .collect();
            let Some(next_ids) = reorder_ids_for_target(&current_ids, &dragged_id, &target) else {
                return;
            };

            let request = SetListPlacementRequest {
                list_ids: next_ids.clone(),
                parent_list_id: None,
                container_id: None,
            };
            let dragged_id_for_mutation = dragged_id.clone();
            let target_for_mutation = target.clone();
            let client = client.clone();
            run_optimistic_mutation(
                root_lists,
                move |lists| {
                    let current_ids: Vec<String> =
                        lists.iter().map(|list| list.id.clone()).collect();
                    let Some(next_ids) = reorder_ids_for_target(
                        &current_ids,
                        &dragged_id_for_mutation,
                        &target_for_mutation,
                    ) else {
                        return false;
                    };
                    apply_reorder(lists, &next_ids, |list| list.id.as_str())
                },
                move || async move { api::reorder_lists(&client, &request).await },
                move |error| toast.push(error.to_string(), ToastKind::Error),
            );
        })
    };

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-4">{move_tr!("home-title")}</h2>

            // Tag filter bar
            <Suspense fallback=|| view! {}>
                {move || if let Some(Ok(tags)) = tags_res.get() {
                    if !tags.is_empty() {
                        let tag_ids: Vec<String> = tags.iter().map(|tag| tag.id.clone()).collect();
                        let filter_options = build_tag_filter_options(&tags, &tag_ids);
                        view! {
                            <TagFilterBar
                                tags=filter_options
                                active_tag_id=active_tag_filter
                                on_select=set_active_tag_filter
                            />
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }
                } else {
                    view! {}.into_any()
                }}
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
                                            let drop_target = DropTarget::before(c.id.clone());
                                            let drop_target_for_marker = drop_target.clone();
                                            let drop_target_for_surface = drop_target.clone();
                                            let drag_id = c.id.clone();
                                            let drag_id_for_handle = drag_id.clone();
                                            let drag_id_for_shell = drag_id.clone();
                                            let drag_id_for_surface = drag_id.clone();
                                            let cid = c.id.clone();
                                            let cid_del = c.id.clone();
                                            let client_del = client_rc.clone();
                                            let client_pn = client_rc.clone();
                                            view! {
                                                <div class="flex flex-col gap-2">
                                                    <ReorderDropTarget
                                                        dnd_state=root_container_dnd_state
                                                        target=drop_target_for_marker
                                                        on_drop=on_root_container_drop.clone()
                                                    />
                                                    <DragShell dnd_state=root_container_dnd_state dragged_id=drag_id_for_shell>
                                                        <DragHandleButton
                                                            dnd_state=root_container_dnd_state
                                                            dragged_id=drag_id_for_handle
                                                            label=DragHandleLabel::Reorder
                                                        />
                                                        <DragSurface
                                                            dnd_state=root_container_dnd_state
                                                            dragged_id=drag_id_for_surface
                                                            hover_target=drop_target_for_surface
                                                        >
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
                                                        </DragSurface>
                                                    </DragShell>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                        <ReorderDropTarget
                                            dnd_state=root_container_dnd_state
                                            target=DropTarget::end()
                                            on_drop=on_root_container_drop
                                        />
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
                                let can_reorder_root_lists = filter.is_none();
                                view! {
                                    <div>
                                        {if !rc_list.is_empty() {
                                            view! {
                                                <h3 class="text-sm font-semibold text-base-content/60 mb-2 uppercase tracking-wide">{move_tr!("home-lists")}</h3>
                                            }.into_any()
                                        } else { view! {}.into_any() }}
                                        <div class="flex flex-col gap-3">
                                            {filtered.into_iter().map(|list| {
                                                let drag_id = list.id.clone();
                                                let drop_target = DropTarget::before(list.id.clone());
                                                let drop_target_for_marker = drop_target.clone();
                                                let drop_target_for_surface = drop_target.clone();
                                                let drag_id_for_handle = drag_id.clone();
                                                let drag_id_for_shell = drag_id.clone();
                                                let drag_id_for_surface = drag_id.clone();
                                                let list_id = list.id.clone();
                                                let list_name = list.name.clone();
                                                let lid_del = list.id.clone();
                                                let list_tag_ids: Vec<String> = all_links
                                                    .iter()
                                                    .filter(|l| l.list_id == list.id)
                                                    .map(|l| l.tag_id.clone())
                                                    .collect();
                                                let tags_for_card = all_tags.clone();
                                                let tog = on_list_tag_toggle.clone();
                                                let tag_cb = Callback::new(move |tag_id: String| tog.run((list_id.clone(), tag_id)));
                                                view! {
                                                    <div class="flex flex-col gap-2">
                                                        {if can_reorder_root_lists {
                                                            view! {
                                                                <ReorderDropTarget
                                                                    dnd_state=root_list_dnd_state
                                                                    target=drop_target_for_marker
                                                                    on_drop=on_root_list_drop.clone()
                                                                />
                                                            }.into_any()
                                                        } else {
                                                            view! {}.into_any()
                                                        }}
                                                        <DragShell dnd_state=root_list_dnd_state dragged_id=drag_id_for_shell>
                                                            {if can_reorder_root_lists {
                                                                view! {
                                                                    <DragHandleButton
                                                                        dnd_state=root_list_dnd_state
                                                                        dragged_id=drag_id_for_handle
                                                                        label=DragHandleLabel::Reorder
                                                                    />
                                                                }.into_any()
                                                            } else {
                                                                view! {}.into_any()
                                                            }}
                                                            <DragSurface
                                                                dnd_state=root_list_dnd_state
                                                                dragged_id=drag_id_for_surface
                                                                hover_target=drop_target_for_surface
                                                            >
                                                                <ListCard
                                                                    list
                                                                    all_tags=tags_for_card
                                                                    list_tag_ids
                                                                    on_tag_toggle=tag_cb
                                                                    on_delete=Callback::new(move |_: String| {
                                                                        pending_delete.set(Some((lid_del.clone(), list_name.clone())));
                                                                    })
                                                                />
                                                            </DragSurface>
                                                        </DragShell>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                            {if can_reorder_root_lists {
                                                view! {
                                                    <ReorderDropTarget
                                                        dnd_state=root_list_dnd_state
                                                        target=DropTarget::end()
                                                        on_drop=on_root_list_drop
                                                    />
                                                }.into_any()
                                            } else {
                                                view! {}.into_any()
                                            }}
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
