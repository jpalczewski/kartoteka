use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::app::{ToastContext, ToastKind};
use crate::components::comments::CommentSection;
use crate::components::common::breadcrumbs::Breadcrumbs;
use crate::components::common::dnd::{DetachDropZone, ReorderDropTarget};
use crate::components::common::editable_text::EditableText;
use crate::components::common::loading::LoadingSpinner;
use crate::components::lists::{container_card::ContainerCard, list_card::ListCard};
use crate::context::GlobalRefresh;
use crate::server_fns::containers::{
    get_container_data, move_container, rename_container, reorder_containers,
};
use crate::server_fns::lists::{move_list, reorder_lists};
use crate::state::dnd::{DndState, DropTarget, EntityKind};

fn container_status_icon(status: Option<&str>) -> &'static str {
    match status {
        None => "📁",
        Some("active") => "🚀",
        Some("done") => "✅",
        Some("paused") => "⏸️",
        _ => "📁",
    }
}

#[component]
pub fn ContainerPage() -> impl IntoView {
    let params = use_params_map();
    let container_id = Signal::derive(move || params.read().get("id").unwrap_or_default());
    let global_refresh = use_context::<GlobalRefresh>().expect("GlobalRefresh missing");
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let (refresh, set_refresh) = signal(0u32);

    let data_res = Resource::new(
        move || (container_id.get(), global_refresh.get(), refresh.get()),
        |(id, _, _)| get_container_data(id),
    );

    // Single state — lists and containers share it; handlers branch on kind.
    let dnd_state: RwSignal<DndState> = RwSignal::new(DndState::default());

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <Transition fallback=|| view! { <LoadingSpinner/> }>
                {move || data_res.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="text-error">"Błąd: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(data) => {
                        let icon = container_status_icon(data.container.status.as_deref());
                        let name = data.container.name.clone();
                        let desc = data.container.description.clone();
                        let ancestors = data.ancestors.clone();
                        let lists = data.lists.clone();
                        let children = data.children.clone();
                        let parent_id = data.container.parent_container_id.clone();
                        let current_id = data.container.id.clone();
                        let container_id_for_rename = data.container.id.clone();
                        let container_id_for_desc = data.container.id.clone();
                        let desc_for_rename = data.container.description.clone();
                        let name_for_desc = name.clone();
                        let child_ids: Vec<String> = children.iter().map(|c| c.id.clone())
                            .chain(lists.iter().map(|l| l.id.clone()))
                            .collect();
                        let child_container_ids: Vec<String> = children.iter().map(|c| c.id.clone()).collect();
                        let child_list_ids: Vec<String> = lists.iter().map(|l| l.id.clone()).collect();

                        // Detach visible when dragged entity is a direct child of this container.
                        let detach_visible = {
                            let child_ids = child_ids.clone();
                            Signal::derive(move || dnd_state.with(|s| {
                                s.dragged_id().map(|id| child_ids.iter().any(|c| c == id)).unwrap_or(false)
                            }))
                        };
                        let parent_for_detach = parent_id.clone();
                        let on_detach = Callback::new(move |_| {
                            let Some((kind, id)) = dnd_state.with_untracked(|s| {
                                s.dragged.as_ref().map(|d| (d.kind, d.id.clone()))
                            }) else { return };
                            let ctr = parent_for_detach.clone();
                            leptos::task::spawn_local(async move {
                                let result = match kind {
                                    EntityKind::Container => move_container(id, ctr).await.map(|_| ()),
                                    EntityKind::List => move_list(id, ctr, None).await.map(|_| ()),
                                };
                                match result {
                                    Ok(()) => global_refresh.bump(),
                                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                }
                            });
                        });

                        // Drop on container card: Container → reparent, List → attach.
                        let current_for_container_drop = current_id.clone();
                        let child_container_ids_drop = child_container_ids.clone();
                        let on_container_nest = Callback::new(move |target: DropTarget| {
                            let Some(nest_id) = target.nest_id().map(str::to_string) else { return };
                            let Some((kind, id)) = dnd_state.with_untracked(|s| {
                                s.dragged.as_ref().map(|d| (d.kind, d.id.clone()))
                            }) else { return };
                            if id == nest_id { return; }
                            leptos::task::spawn_local(async move {
                                let result = match kind {
                                    EntityKind::Container => move_container(id, Some(nest_id)).await.map(|_| ()),
                                    EntityKind::List => move_list(id, Some(nest_id), None).await.map(|_| ()),
                                };
                                match result {
                                    Ok(()) => global_refresh.bump(),
                                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                }
                            });
                            let _ = (&current_for_container_drop, &child_container_ids_drop);
                        });

                        // Drop on list card: List → make sublist. Container → ignore.
                        let on_list_nest = Callback::new(move |target: DropTarget| {
                            let Some(nest_id) = target.nest_id().map(str::to_string) else { return };
                            let Some((kind, id)) = dnd_state.with_untracked(|s| {
                                s.dragged.as_ref().map(|d| (d.kind, d.id.clone()))
                            }) else { return };
                            if kind != EntityKind::List || id == nest_id { return; }
                            leptos::task::spawn_local(async move {
                                match move_list(id, None, Some(nest_id)).await {
                                    Ok(_) => global_refresh.bump(),
                                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                }
                            });
                        });

                        // Reorder drop for containers (children at same level as `current`).
                        let child_container_ids_for_reorder = child_container_ids.clone();
                        let current_for_reorder = current_id.clone();
                        let on_container_reorder = Callback::new(move |target: DropTarget| {
                            let Some((kind, dragged_id)) = dnd_state.with_untracked(|s| {
                                s.dragged.as_ref().map(|d| (d.kind, d.id.clone()))
                            }) else { return };
                            if kind != EntityKind::Container { return; }
                            let mut ids = child_container_ids_for_reorder.clone();
                            ids.retain(|x| x != &dragged_id);
                            let insert_at = match &target {
                                DropTarget::Before(b) => ids.iter().position(|x| x == b).unwrap_or(ids.len()),
                                DropTarget::End => ids.len(),
                                _ => return,
                            };
                            ids.insert(insert_at, dragged_id);
                            let parent = Some(current_for_reorder.clone());
                            leptos::task::spawn_local(async move {
                                match reorder_containers(parent, ids).await {
                                    Ok(()) => global_refresh.bump(),
                                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                }
                            });
                        });

                        // Reorder drop for lists at this container level.
                        let child_list_ids_for_reorder = child_list_ids.clone();
                        let current_for_list_reorder = current_id.clone();
                        let on_list_reorder = Callback::new(move |target: DropTarget| {
                            let Some((kind, dragged_id)) = dnd_state.with_untracked(|s| {
                                s.dragged.as_ref().map(|d| (d.kind, d.id.clone()))
                            }) else { return };
                            if kind != EntityKind::List { return; }
                            let mut ids = child_list_ids_for_reorder.clone();
                            ids.retain(|x| x != &dragged_id);
                            let insert_at = match &target {
                                DropTarget::Before(b) => ids.iter().position(|x| x == b).unwrap_or(ids.len()),
                                DropTarget::End => ids.len(),
                                _ => return,
                            };
                            ids.insert(insert_at, dragged_id);
                            let ctr = Some(current_for_list_reorder.clone());
                            leptos::task::spawn_local(async move {
                                match reorder_lists(ctr, None, ids).await {
                                    Ok(()) => global_refresh.bump(),
                                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                }
                            });
                        });

                        view! {
                            <div class="flex flex-col gap-6">
                                <DetachDropZone
                                    dnd_state=dnd_state
                                    visible=detach_visible
                                    on_drop=on_detach
                                    label="Upuść tutaj, aby wyjąć do rodzica"
                                />

                                <Breadcrumbs crumbs=ancestors current=name.clone() />

                                // Header
                                <div class="flex items-center gap-3">
                                    <span class="text-3xl">{icon}</span>
                                    <div class="flex flex-col gap-1">
                                        <EditableText
                                            value=name.clone()
                                            on_save=Callback::new(move |new_name: String| {
                                                let lid = container_id_for_rename.clone();
                                                let current_desc = desc_for_rename.clone();
                                                leptos::task::spawn_local(async move {
                                                    match rename_container(lid, new_name, current_desc).await {
                                                        Ok(_) => set_refresh.update(|n| *n += 1),
                                                        Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                                    }
                                                });
                                            })
                                            class="text-2xl font-bold cursor-pointer hover:underline decoration-dotted"
                                        />
                                        <EditableText
                                            value=desc.clone().unwrap_or_default()
                                            on_save=Callback::new(move |new_desc: String| {
                                                let lid = container_id_for_desc.clone();
                                                let current_name = name_for_desc.clone();
                                                let desc_opt = if new_desc.trim().is_empty() { None } else { Some(new_desc) };
                                                leptos::task::spawn_local(async move {
                                                    match rename_container(lid, current_name, desc_opt).await {
                                                        Ok(_) => set_refresh.update(|n| *n += 1),
                                                        Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                                    }
                                                });
                                            })
                                            multiline=true
                                            placeholder="Dodaj opis..."
                                            class="text-base-content/60 text-sm cursor-pointer hover:underline decoration-dotted"
                                        />
                                    </div>
                                </div>

                                // Child containers
                                {if !children.is_empty() {
                                    view! {
                                        <div>
                                            <h3 class="text-sm font-semibold text-base-content/60 mb-2 uppercase tracking-wide">
                                                "Subkontenerów (" {children.len()} ")"
                                            </h3>
                                            <div class="flex flex-col gap-1">
                                                {children.into_iter().map(|child| {
                                                    let cid = child.id.clone();
                                                    view! {
                                                        <ReorderDropTarget
                                                            dnd_state=dnd_state
                                                            target=DropTarget::Before(cid)
                                                            on_drop=on_container_reorder
                                                        />
                                                        <ContainerCard
                                                            container=child
                                                            dnd_state=dnd_state
                                                            on_nest_drop=on_container_nest
                                                        />
                                                    }
                                                }).collect::<Vec<_>>()}
                                                <ReorderDropTarget
                                                    dnd_state=dnd_state
                                                    target=DropTarget::End
                                                    on_drop=on_container_reorder
                                                    label="Upuść na koniec"
                                                />
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {}.into_any()
                                }}

                                // Lists in this container
                                {if lists.is_empty() {
                                    view! {
                                        <div class="text-center text-base-content/50 py-4">
                                            "Brak list w tym kontenerze."
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div>
                                            <h3 class="text-sm font-semibold text-base-content/60 mb-2 uppercase tracking-wide">
                                                "Listy (" {lists.len()} ")"
                                            </h3>
                                            <div class="flex flex-col gap-1">
                                                {lists.into_iter().map(|list| {
                                                    let lid = list.id.clone();
                                                    view! {
                                                        <ReorderDropTarget
                                                            dnd_state=dnd_state
                                                            target=DropTarget::Before(lid)
                                                            on_drop=on_list_reorder
                                                        />
                                                        <ListCard
                                                            list=list
                                                            dnd_state=dnd_state
                                                            on_nest_drop=on_list_nest
                                                        />
                                                    }
                                                }).collect::<Vec<_>>()}
                                                <ReorderDropTarget
                                                    dnd_state=dnd_state
                                                    target=DropTarget::End
                                                    on_drop=on_list_reorder
                                                    label="Upuść na koniec"
                                                />
                                            </div>
                                        </div>
                                    }.into_any()
                                }}

                                // Comments
                                <div>
                                    <h3 class="text-sm font-semibold text-base-content/60 mb-2 uppercase tracking-wide">
                                        "Komentarze"
                                    </h3>
                                    <CommentSection
                                        entity_type="container"
                                        entity_id=container_id
                                    />
                                </div>
                            </div>
                        }.into_any()
                    }
                })}
            </Transition>
        </div>
    }
}
