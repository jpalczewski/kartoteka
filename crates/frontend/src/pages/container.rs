use leptos::prelude::*;
use leptos_fluent::{I18n, move_tr};
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::app::{ToastContext, ToastKind};
use crate::components::comments::CommentSection;
use crate::components::common::breadcrumbs::Breadcrumbs;
use crate::components::common::container_selector_dropdown::ContainerSelectorDropdown;
use crate::components::common::delete_entity_modal::{DeleteEntityModal, DeleteTarget};
use crate::components::common::dnd::{DetachDropZone, ReorderDropTarget};
use crate::components::common::editable_text::EditableText;
use crate::components::common::loading::LoadingSpinner;
use crate::components::lists::{
    container_card::ContainerCard, create_entity_input::CreateEntityInput,
    list_preview::ListPreview,
};
use crate::components::ui::{EntityIcon, Icon, IconButton, IconButtonSize, IconButtonVariant};
use crate::server_fns::containers::{
    archive_container, create_container, get_container_data, get_containers_for_move,
    move_container, rename_container, reorder_containers,
};
use crate::server_fns::lists::{archive_list, create_list, move_list, reorder_lists};
use crate::state::dnd::{DndState, DropTarget, EntityKind};
use crate::state::invalidation::{mutate, mutate_with, use_invalidation};
use kartoteka_shared::{ChangeTopic, default_container_icon};

#[component]
pub fn ContainerPage() -> impl IntoView {
    let i18n = expect_context::<I18n>();
    let params = use_params_map();
    let container_id = Signal::derive(move || params.read().get("id").unwrap_or_default());
    // Delete of this container or one of its children goes through the shared flow.
    let delete_target: RwSignal<Option<DeleteTarget>> = RwSignal::new(None);
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let invalidation = use_invalidation();
    let navigate = StoredValue::new(use_navigate());
    let expand_all_label = StoredValue::new(i18n.tr("lists-expand-all"));
    let collapse_all_label = StoredValue::new(i18n.tr("lists-collapse-all"));
    let (expand_all, set_expand_all) = signal(false);

    let container_dropdown_open = RwSignal::new(false);
    // container_id in key_fn — Leptos tracks reactivity only in key_fn, not in the async block.
    let containers_res = Resource::new(
        move || (container_dropdown_open.get(), container_id.get()),
        |(open, cid)| async move {
            if open {
                get_containers_for_move(Some(cid), true).await
            } else {
                Ok(vec![])
            }
        },
    );
    let on_move_container_to_parent = Callback::new(move |pid: String| {
        let cid = container_id.get();
        container_dropdown_open.set(false);
        mutate(move_container(cid, Some(pid)), &[ChangeTopic::Containers]);
    });

    let data_res = Resource::new(
        move || {
            (
                container_id.get(),
                invalidation.track(&[
                    ChangeTopic::Containers,
                    ChangeTopic::Lists,
                    ChangeTopic::Comments,
                ]),
            )
        },
        |(id, _)| get_container_data(id),
    );

    // Single state — lists and containers share it; handlers branch on kind.
    let dnd_state: RwSignal<DndState> = RwSignal::new(DndState::default());

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <DeleteEntityModal
                target=delete_target
                on_deleted=Callback::new(move |target: DeleteTarget| {
                    // Deleted this container — nothing left to show here. Children
                    // vanish through the invalidation done by the modal.
                    if target.id == container_id.get_untracked() {
                        navigate.with_value(|nav| nav("/", Default::default()));
                    }
                })
            />

            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || data_res.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="text-error">"Błąd: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(data) => {
                        let container_icon_raw = data.container.icon.clone();
                        let container_icon_fallback = default_container_icon(data.container.status.is_some());
                        let name = data.container.name.clone();
                        let desc = data.container.description.clone();
                        let ancestors = data.ancestors.clone();
                        let lists = data.lists.clone();
                        let children = data.children.clone();
                        let comments_payload = data.comments.clone();
                        let parent_id = data.container.parent_container_id.clone();
                        let current_id = data.container.id.clone();
                        let parent_name_sv = StoredValue::new(
                            data.ancestors.last().map(|(_, n)| n.clone()),
                        );
                        let current_id_for_move_sv = StoredValue::new(data.container.id.clone());
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
                                    Ok(()) => invalidation.invalidate_many(&[
                                        ChangeTopic::Containers,
                                        ChangeTopic::Lists,
                                    ]),
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
                                    Ok(()) => invalidation.invalidate_many(&[
                                        ChangeTopic::Containers,
                                        ChangeTopic::Lists,
                                    ]),
                                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                }
                            });
                            let _ = (&current_for_container_drop, &child_container_ids_drop);
                        });

                        // Drop on list card: List → make sublist. Container → ignore.
                        let _on_list_nest = Callback::new(move |target: DropTarget| {
                            let Some(nest_id) = target.nest_id().map(str::to_string) else { return };
                            let Some((kind, id)) = dnd_state.with_untracked(|s| {
                                s.dragged.as_ref().map(|d| (d.kind, d.id.clone()))
                            }) else { return };
                            if kind != EntityKind::List || id == nest_id { return; }
                            leptos::task::spawn_local(async move {
                                match move_list(id, None, Some(nest_id)).await {
                                    Ok(_) => invalidation.invalidate(ChangeTopic::Lists),
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
                                    Ok(()) => invalidation.invalidate(ChangeTopic::Containers),
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
                                    Ok(()) => invalidation.invalidate(ChangeTopic::Lists),
                                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                }
                            });
                        });

                        let on_create_list = Callback::new(move |req| {
                            mutate(create_list(req), &[ChangeTopic::Lists, ChangeTopic::Containers]);
                        });
                        let on_create_container = Callback::new(move |req| {
                            mutate(create_container(req), &[ChangeTopic::Containers]);
                        });

                        // Callbacks for child containers
                        let archived_msg = i18n.tr("home-container-archived");
                        let on_archive_child_container = {
                            let msg = archived_msg.clone();
                            Callback::new(move |id: String| {
                                let msg = msg.clone();
                                mutate_with(
                                    archive_container(id),
                                    &[ChangeTopic::Containers],
                                    move |()| toast.push(msg, ToastKind::Success),
                                );
                            })
                        };

                        // Names are needed for the delete dialog; cards only pass ids.
                        let child_container_names: std::collections::HashMap<String, String> =
                            children.iter().map(|c| (c.id.clone(), c.name.clone())).collect();
                        let child_list_names: std::collections::HashMap<String, String> =
                            lists.iter().map(|l| (l.id.clone(), l.name.clone())).collect();

                        let on_delete_child_container = Callback::new(move |id: String| {
                            let name = child_container_names.get(&id).cloned().unwrap_or_default();
                            delete_target.set(Some(DeleteTarget::container(id, name)));
                        });

                        // Callbacks for lists
                        let on_archive_list_cb = Callback::new(move |id: String| {
                            mutate(archive_list(id), &[ChangeTopic::Lists]);
                        });

                        let on_delete_list_cb = Callback::new(move |id: String| {
                            let name = child_list_names.get(&id).cloned().unwrap_or_default();
                            delete_target.set(Some(DeleteTarget::list(id, name)));
                        });

                        // Callbacks for the current container itself (archive/delete → navigate away)
                        let cid_for_archive = data.container.id.clone();
                        let cid_for_delete = data.container.id.clone();
                        let name_for_delete = name.clone();

                        let on_archive_self = {
                            let msg = archived_msg.clone();
                            Callback::new(move |_: ()| {
                                let id = cid_for_archive.clone();
                                let msg = msg.clone();
                                mutate_with(
                                    archive_container(id),
                                    &[ChangeTopic::Containers],
                                    move |()| {
                                        toast.push(msg, ToastKind::Success);
                                        navigate.with_value(|nav| nav("/", Default::default()));
                                    },
                                );
                            })
                        };

                        let on_request_delete_self = Callback::new(move |_: ()| {
                            delete_target.set(Some(DeleteTarget::container(
                                cid_for_delete.clone(),
                                name_for_delete.clone(),
                            )));
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
                                    <EntityIcon raw=container_icon_raw fallback=container_icon_fallback size=30 />
                                    <div class="flex-1 flex flex-col gap-1">
                                        <EditableText
                                            value=name.clone()
                                            on_save=Callback::new(move |new_name: String| {
                                                let lid = container_id_for_rename.clone();
                                                let current_desc = desc_for_rename.clone();
                                                mutate(
                                                    rename_container(lid, new_name, current_desc),
                                                    &[ChangeTopic::Containers],
                                                );
                                            })
                                            class="text-2xl font-bold cursor-pointer hover:underline decoration-dotted"
                                        />
                                        <EditableText
                                            value=desc.clone().unwrap_or_default()
                                            on_save=Callback::new(move |new_desc: String| {
                                                let lid = container_id_for_desc.clone();
                                                let current_name = name_for_desc.clone();
                                                let desc_opt = if new_desc.trim().is_empty() { None } else { Some(new_desc) };
                                                mutate(
                                                    rename_container(lid, current_name, desc_opt),
                                                    &[ChangeTopic::Containers],
                                                );
                                            })
                                            multiline=true
                                            placeholder="Dodaj opis..."
                                            class="text-base-content/60 text-sm cursor-pointer hover:underline decoration-dotted"
                                        />
                                    </div>
                                    <div class="flex gap-1 items-center">
                                        // Reparent/detach UI
                                        {move || {
                                            let opts = containers_res.get().and_then(|r| r.ok()).unwrap_or_default();
                                            if let Some(pname) = parent_name_sv.get_value() {
                                                view! {
                                                    <div class="flex items-center gap-1">
                                                        <span class="text-xs text-base-content/60 flex items-center gap-1">
                                                            <Icon name="folder" size=12 />
                                                            <span class="max-w-24 truncate">{pname}</span>
                                                        </span>
                                                        <IconButton
                                                            icon="x"
                                                            label=move_tr!("lists-detach-from-parent")
                                                            variant=IconButtonVariant::Danger
                                                            on_click=Callback::new(move |_| {
                                                                let cid = current_id_for_move_sv.get_value();
                                                                mutate(
                                                                    move_container(cid, None),
                                                                    &[ChangeTopic::Containers],
                                                                );
                                                            })
                                                        />
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <div class="relative">
                                                        <IconButton
                                                            icon="folder"
                                                            label=move_tr!("lists-move-to-parent")
                                                            on_click=Callback::new(move |_| container_dropdown_open.update(|v| *v = !*v))
                                                        />
                                                        <ContainerSelectorDropdown
                                                            open=container_dropdown_open
                                                            options=opts
                                                            on_select=on_move_container_to_parent
                                                        />
                                                    </div>
                                                }.into_any()
                                            }
                                        }}
                                        <IconButton
                                            icon="archive"
                                            label=move_tr!("lists-archive-container-aria")
                                            size=IconButtonSize::Sm
                                            on_click=Callback::new(move |_| on_archive_self.run(()))
                                        />
                                        <IconButton
                                            icon="trash"
                                            label=move_tr!("lists-delete-container-aria")
                                            variant=IconButtonVariant::Danger
                                            size=IconButtonSize::Sm
                                            testid=Some("container-delete-self")
                                            on_click=Callback::new(move |_| on_request_delete_self.run(()))
                                        />
                                    </div>
                                </div>

                                <CreateEntityInput
                                    parent_container_id=current_id.clone()
                                    on_create_list=on_create_list
                                    on_create_container=on_create_container
                                />

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
                                                            on_archive=on_archive_child_container
                                                            on_delete=on_delete_child_container
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
                                            <div class="flex items-center justify-between mb-2">
                                                <h3 class="text-sm font-semibold text-base-content/60 uppercase tracking-wide">
                                                    "Listy (" {lists.len()} ")"
                                                </h3>
                                                <button
                                                    type="button"
                                                    class="btn btn-ghost btn-xs"
                                                    on:click=move |_| set_expand_all.update(|v| *v = !*v)
                                                >
                                                    {move || if expand_all.get() { collapse_all_label.get_value() } else { expand_all_label.get_value() }}
                                                </button>
                                            </div>
                                            <div class="flex flex-col gap-1">
                                                {lists.into_iter().map(|list| {
                                                    let lid = list.id.clone();
                                                    view! {
                                                        <ReorderDropTarget
                                                            dnd_state=dnd_state
                                                            target=DropTarget::Before(lid)
                                                            on_drop=on_list_reorder
                                                        />
                                                        <ListPreview
                                                            list=list
                                                            on_archive=on_archive_list_cb
                                                            on_delete=on_delete_list_cb
                                                            force_expand=Signal::from(expand_all)
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
                                        initial_payload=comments_payload
                                    />
                                </div>
                            </div>
                        }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}
