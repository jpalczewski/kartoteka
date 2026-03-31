use kartoteka_shared::{
    Container, ContainerDetail, ContainerStatus, CreateContainerRequest, CreateListRequest, List,
    UpdateContainerRequest,
};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::api;
use crate::api::client::GlooClient;
use crate::app::{ToastContext, ToastKind};
use crate::components::common::breadcrumbs::Breadcrumbs;
use crate::components::common::editable_description::EditableDescription;
use crate::components::common::editable_title::EditableTitle;
use crate::components::common::loading::LoadingSpinner;
use crate::components::confirm_delete_modal::ConfirmDeleteModal;
use crate::components::container_card::ContainerCard;
use crate::components::create_entity_input::CreateEntityInput;
use crate::components::list_card::ListCard;

async fn build_breadcrumbs(
    container_id: &str,
    all_containers: &[Container],
) -> Vec<(String, String)> {
    let mut crumbs = Vec::new();
    let mut current_id = Some(container_id.to_string());

    // Walk up the parent chain (max depth guard: 10)
    let mut chain = Vec::new();
    let mut depth = 0;
    while let Some(ref cid) = current_id.clone() {
        if depth > 10 {
            break;
        }
        if let Some(c) = all_containers.iter().find(|c| &c.id == cid) {
            chain.push((c.name.clone(), format!("/containers/{}", c.id)));
            current_id = c.parent_container_id.clone();
        } else {
            break;
        }
        depth += 1;
    }
    chain.reverse();
    // Remove the last item (current page, will be shown as plain text by the page title)
    if !chain.is_empty() {
        chain.pop();
    }
    crumbs.extend(chain);
    crumbs
}

#[component]
pub fn ContainerPage() -> impl IntoView {
    let params = use_params_map();
    let container_id = move || params.read().get("id").unwrap_or_default();

    let client = use_context::<GlooClient>().expect("GlooClient not provided");
    let toast = use_context::<ToastContext>().expect("ToastContext missing");

    let (refresh, set_refresh) = signal(0u32);
    let detail = RwSignal::new(Option::<ContainerDetail>::None);
    let sub_containers = RwSignal::new(Vec::<Container>::new());
    let sub_lists = RwSignal::new(Vec::<List>::new());
    let breadcrumbs = RwSignal::new(Vec::<(String, String)>::new());
    let (loading, set_loading) = signal(true);
    let pending_delete_list = RwSignal::new(Option::<(String, String)>::None);

    // Reactive effect: re-runs whenever container_id or refresh changes.
    // This ensures navigating folder→folder (same route, different :id) re-fetches data.
    Effect::new({
        let client = client.clone();
        move |_| {
            let cid = container_id();
            let _r = refresh.get(); // track refresh signal too

            detail.set(None);
            sub_containers.set(vec![]);
            sub_lists.set(vec![]);
            breadcrumbs.set(vec![]);
            set_loading.set(true);

            let client = client.clone();
            leptos::task::spawn_local(async move {
                if let Ok(det) = api::fetch_container(&client, &cid).await {
                    detail.set(Some(det));
                }
                if let Ok(children) = api::fetch_container_children(&client, &cid).await {
                    sub_containers.set(children.containers);
                    sub_lists.set(children.lists);
                }
                if let Ok(all) = api::fetch_containers(&client).await {
                    let crumbs = build_breadcrumbs(&cid, &all).await;
                    breadcrumbs.set(crumbs);
                }
                set_loading.set(false);
            });
        }
    });

    let is_project = move || {
        detail
            .get()
            .map(|d| d.container.status.is_some())
            .unwrap_or(false)
    };

    let on_create_list = {
        let client = client.clone();
        Callback::new(move |req: CreateListRequest| {
            let cid = container_id();
            let client = client.clone();
            leptos::task::spawn_local(async move {
                match api::create_list(&client, &req).await {
                    Ok(mut list) => {
                        // Move new list into this container
                        let _ = api::move_list_to_container(&client, &list.id, Some(&cid)).await;
                        list.container_id = Some(cid);
                        sub_lists.update(|ls| ls.push(list));
                    }
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
                    Ok(c) => sub_containers.update(|cs| cs.push(c)),
                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                }
            });
        })
    };

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            {move || {
                let crumbs = breadcrumbs.get();
                if !crumbs.is_empty() {
                    view! { <Breadcrumbs crumbs=crumbs /> }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}

            {move || {
                if loading.get() {
                    return view! { <LoadingSpinner/> }.into_any();
                }

                let det = detail.get();
                let Some(det) = det else {
                    return view! { <p class="text-error">"Nie znaleziono kontenera"</p> }.into_any();
                };

                let container = det.container.clone();
                let cid = container.id.clone();
                let cid_name = cid.clone();
                let cid_desc = cid.clone();
                let cid_status = cid.clone();
                let is_proj = container.status.is_some();
                let completed_items = det.completed_items;
                let total_items = det.total_items;
                let completed_lists = det.completed_lists;
                let total_lists_count = det.total_lists;

                let client_inner = use_context::<GlooClient>().expect("GlooClient not provided");

                view! {
                    <div>
                        // Header
                        <div class="mb-4">
                            <div class="flex items-center gap-2 mb-1">
                                <span class="text-2xl">
                                    {if is_proj { "🚀" } else { "📁" }}
                                </span>
                                <EditableTitle
                                    value=container.name.clone()
                                    on_save=Callback::new(move |name: String| {
                                        let cid = cid_name.clone();
                                        let client = use_context::<GlooClient>().expect("GlooClient not provided");
                                        leptos::task::spawn_local(async move {
                                            let req = UpdateContainerRequest {
                                                name: Some(name),
                                                description: None,
                                                status: None,
                                            };
                                            match api::update_container(&client, &cid, &req).await {
                                                Ok(c) => detail.update(|d| {
                                                    if let Some(det) = d {
                                                        det.container.name = c.name;
                                                    }
                                                }),
                                                Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                            }
                                        });
                                    })
                                />
                            </div>

                            <EditableDescription
                                value=container.description.clone()
                                on_save=Callback::new(move |desc: Option<String>| {
                                    let cid = cid_desc.clone();
                                    let client = use_context::<GlooClient>().expect("GlooClient not provided");
                                    leptos::task::spawn_local(async move {
                                        let req = UpdateContainerRequest {
                                            name: None,
                                            description: Some(desc),
                                            status: None,
                                        };
                                        let _ = api::update_container(&client, &cid, &req).await;
                                    });
                                })
                            />
                        </div>

                        // Project status + progress
                        {if is_proj {
                            let status_str = match &container.status {
                                Some(ContainerStatus::Active) => "active",
                                Some(ContainerStatus::Done) => "done",
                                Some(ContainerStatus::Paused) => "paused",
                                None => "active",
                            };
                            let pct = if total_items > 0 {
                                (completed_items as f32 / total_items as f32 * 100.0) as u32
                            } else { 0 };

                            view! {
                                <div class="mb-4 p-4 bg-base-200 rounded-lg">
                                    // Status selector
                                    <div class="flex items-center gap-2 mb-3">
                                        <span class="text-sm font-medium">"Status:"</span>
                                        <select
                                            class="select select-sm select-bordered"
                                            on:change=move |ev| {
                                                let val = event_target_value(&ev);
                                                let new_status = match val.as_str() {
                                                    "active" => Some(ContainerStatus::Active),
                                                    "done" => Some(ContainerStatus::Done),
                                                    "paused" => Some(ContainerStatus::Paused),
                                                    _ => None,
                                                };
                                                let cid = cid_status.clone();
                                                let client = use_context::<GlooClient>().expect("GlooClient not provided");
                                                leptos::task::spawn_local(async move {
                                                    let req = UpdateContainerRequest {
                                                        name: None,
                                                        description: None,
                                                        status: Some(new_status),
                                                    };
                                                    match api::update_container(&client, &cid, &req).await {
                                                        Ok(c) => detail.update(|d| {
                                                            if let Some(det) = d {
                                                                det.container.status = c.status;
                                                            }
                                                        }),
                                                        Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                                    }
                                                });
                                            }
                                        >
                                            <option value="active" selected=move || status_str == "active">"Aktywny"</option>
                                            <option value="done" selected=move || status_str == "done">"Ukończony"</option>
                                            <option value="paused" selected=move || status_str == "paused">"Wstrzymany"</option>
                                        </select>
                                    </div>

                                    // Item-level progress
                                    <div class="mb-2">
                                        <div class="flex justify-between text-xs text-base-content/60 mb-1">
                                            <span>"Zadania: " {completed_items} "/" {total_items}</span>
                                            <span>{pct}"%"</span>
                                        </div>
                                        <progress class="progress progress-primary w-full" value=completed_items max=total_items.max(1)></progress>
                                    </div>

                                    // List-level progress
                                    <div class="text-xs text-base-content/60">
                                        "Listy ukończone: " {completed_lists} "/" {total_lists_count}
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}

                        // Create entity input
                        <CreateEntityInput
                            parent_container_id=container.id.clone()
                            show_container_options=!is_project()
                            on_create_list=on_create_list
                            on_create_container=on_create_container
                        />

                        // Sub-containers
                        {move || {
                            let scs = sub_containers.get();
                            if scs.is_empty() {
                                view! {}.into_any()
                            } else {
                                let client_sc = client_inner.clone();
                                view! {
                                    <div class="mb-4">
                                        <h3 class="text-sm font-semibold text-base-content/60 mb-2 uppercase tracking-wide">"Kontenery"</h3>
                                        <div class="flex flex-col gap-3">
                                            {scs.into_iter().map(|c| {
                                                let cid_del = c.id.clone();
                                                let client_del = client_sc.clone();
                                                view! {
                                                    <ContainerCard
                                                        container=c
                                                        on_delete=Callback::new(move |_: String| {
                                                            let cid = cid_del.clone();
                                                            let client_d = client_del.clone();
                                                            leptos::task::spawn_local(async move {
                                                                match api::delete_container(&client_d, &cid).await {
                                                                    Ok(()) => {
                                                                        sub_containers.update(|cs| cs.retain(|c| c.id != cid));
                                                                        toast.push("Kontener usunięty".into(), ToastKind::Success);
                                                                    }
                                                                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                                                }
                                                            });
                                                        })
                                                    />
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                }.into_any()
                            }
                        }}

                        // Lists in container
                        {move || {
                            let lists = sub_lists.get();
                            if lists.is_empty() {
                                view! { <div class="text-center text-base-content/50 py-8">"Brak list w tym kontenerze."</div> }.into_any()
                            } else {
                                view! {
                                    <div class="mb-4">
                                        <h3 class="text-sm font-semibold text-base-content/60 mb-2 uppercase tracking-wide">"Listy"</h3>
                                        <div class="flex flex-col gap-3">
                                            {lists.into_iter().map(|list| {
                                                let lid_del = list.id.clone();
                                                let lname_del = list.name.clone();
                                                view! {
                                                    <ListCard
                                                        list
                                                        on_delete=Callback::new(move |_: String| {
                                                            pending_delete_list.set(Some((lid_del.clone(), lname_del.clone())));
                                                        })
                                                    />
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                }.into_any()
                            }
                        }}

                        // Delete list modal
                        {move || pending_delete_list.get().map(|(lid, lname)| {
                            let lid_confirm = lid.clone();
                            let client_modal = use_context::<GlooClient>().expect("GlooClient not provided");
                            view! {
                                <ConfirmDeleteModal
                                    list_id=lid
                                    list_name=lname
                                    on_confirm=Callback::new(move |_| {
                                        let lid = lid_confirm.clone();
                                        let client_m = client_modal.clone();
                                        leptos::task::spawn_local(async move {
                                            sub_lists.update(|ls| ls.retain(|l| l.id != lid));
                                            pending_delete_list.set(None);
                                            match api::delete_list(&client_m, &lid).await {
                                                Ok(()) => toast.push("Lista usunięta".into(), ToastKind::Success),
                                                Err(e) => {
                                                    set_refresh.update(|n| *n += 1);
                                                    toast.push(e.to_string(), ToastKind::Error);
                                                }
                                            }
                                        });
                                    })
                                    on_cancel=Callback::new(move |_| pending_delete_list.set(None))
                                />
                            }
                        })}
                    </div>
                }.into_any()
            }}
        </div>
    }
}
