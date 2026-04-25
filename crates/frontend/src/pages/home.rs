use kartoteka_shared::types::{CreateContainerRequest, CreateListRequest};
use leptos::prelude::*;

use crate::app::{ToastContext, ToastKind};
use crate::components::common::{
    confirm_modal::{ConfirmModal, ConfirmVariant},
    loading::LoadingSpinner,
};
use crate::components::home::{
    pinned_section::PinnedSection, recent_section::RecentSection, root_section::RootSection,
};
use crate::components::lists::create_entity_input::CreateEntityInput;
use crate::components::tags::tag_badge::TagBadge;
use crate::context::GlobalRefresh;
use crate::server_fns::{
    containers::{create_container, delete_container, toggle_container_pin},
    home::{get_archived_lists, get_home_data},
    lists::{archive_list, create_list, delete_list},
    tags::{assign_tag_to_list, get_all_tags, get_list_tag_links, remove_tag_from_list},
};

#[component]
pub fn HomePage() -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");

    // Refresh trigger — incrementing causes all Resources to refetch
    let (refresh, set_refresh) = signal(0u32);
    let global_refresh = use_context::<GlobalRefresh>().expect("GlobalRefresh missing");

    // Tag filter — which tag_id is active (None = no filter)
    let (active_tag_filter, set_active_tag_filter) = signal(Option::<String>::None);

    // Pending delete state — (list_id, list_name)
    let pending_delete: RwSignal<Option<(String, String)>> = RwSignal::new(None);

    // Resources — fetched server-side at initial render, refetched on refresh signal
    let home_res = Resource::new(
        move || (refresh.get(), global_refresh.get()),
        |_| get_home_data(),
    );
    let archived_res = Resource::new(
        move || (refresh.get(), global_refresh.get()),
        |_| get_archived_lists(),
    );
    let tags_res = Resource::new(|| (), |_| get_all_tags());
    let tag_links_res = Resource::new(
        move || (refresh.get(), global_refresh.get()),
        |_| get_list_tag_links(),
    );

    // ── Mutation callbacks ─────────────────────────────────────────────

    let on_create_list = Callback::new(move |req: CreateListRequest| {
        leptos::task::spawn_local(async move {
            match create_list(req).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_create_container = Callback::new(move |req: CreateContainerRequest| {
        leptos::task::spawn_local(async move {
            match create_container(req).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_delete_list_confirmed = Callback::new(move |list_id: String| {
        leptos::task::spawn_local(async move {
            match delete_list(list_id).await {
                Ok(_) => {
                    pending_delete.set(None);
                    set_refresh.update(|n| *n += 1);
                    toast.push("Lista usunięta.".to_string(), ToastKind::Success);
                }
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_delete_container = Callback::new(move |container_id: String| {
        leptos::task::spawn_local(async move {
            match delete_container(container_id).await {
                Ok(_) => {
                    set_refresh.update(|n| *n += 1);
                    toast.push("Folder usunięty.".to_string(), ToastKind::Success);
                }
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_pin_container = Callback::new(move |container_id: String| {
        leptos::task::spawn_local(async move {
            match toggle_container_pin(container_id).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_tag_toggle = Callback::new(move |(list_id, tag_id): (String, String)| {
        leptos::task::spawn_local(async move {
            let has_tag = tag_links_res
                .get()
                .and_then(|r| r.ok())
                .unwrap_or_default()
                .iter()
                .any(|l| l.list_id == list_id && l.tag_id == tag_id);
            let result = if has_tag {
                remove_tag_from_list(list_id, tag_id).await
            } else {
                assign_tag_to_list(list_id, tag_id).await
            };
            if let Err(e) = result {
                toast.push(e.to_string(), ToastKind::Error);
            }
            set_refresh.update(|n| *n += 1);
        });
    });

    let on_restore_list = Callback::new(move |list_id: String| {
        leptos::task::spawn_local(async move {
            match archive_list(list_id).await {
                Ok(_) => {
                    set_refresh.update(|n| *n += 1);
                    toast.push("Lista przywrócona.".to_string(), ToastKind::Success);
                }
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-4">"Strona główna"</h2>

            {move || pending_delete.get().map(|(lid, lname)| {
                let lid_confirm = lid.clone();
                view! {
                    <ConfirmModal
                        open=Signal::derive(move || pending_delete.get().is_some())
                        title="Usuń listę".to_string()
                        message=format!("Czy na pewno chcesz usunąć listę \"{}\"?", lname)
                        confirm_label="Usuń".to_string()
                        variant=ConfirmVariant::Danger
                        on_close=Callback::new(move |_| pending_delete.set(None))
                        on_confirm=Callback::new(move |_| {
                            pending_delete.set(None);
                            on_delete_list_confirmed.run(lid_confirm.clone());
                        })
                    />
                }
            })}

            // Tag filter bar
            <Transition fallback=|| view! {}>
                {move || tags_res.get().map(|result| match result {
                    Ok(tags) if !tags.is_empty() => {
                        view! {
                            <div class="flex flex-wrap gap-1 mb-3">
                                {tags.into_iter().map(|tag| {
                                    let tid = tag.id.clone();
                                    let is_active = active_tag_filter.get().as_deref() == Some(&tid);
                                    view! {
                                        <TagBadge
                                            tag=tag
                                            active=is_active
                                            on_click=Callback::new(move |id: String| {
                                                set_active_tag_filter.update(|f| {
                                                    *f = if f.as_deref() == Some(&id) { None } else { Some(id) };
                                                });
                                            })
                                        />
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    }
                    _ => view! {}.into_any(),
                })}
            </Transition>

            // Create form
            <CreateEntityInput
                show_container_options=true
                on_create_list=on_create_list
                on_create_container=on_create_container
            />

            // Main content: sections
            <Transition fallback=|| view! { <LoadingSpinner/> }>
                {move || {
                    let home = home_res.get();
                    let links = tag_links_res.get();
                    let all_tags = tags_res.get();

                    match (home, links, all_tags) {
                        (Some(Ok(data)), Some(Ok(all_links)), Some(Ok(tags))) => {
                            // Pre-extract list names for delete modal
                            let all_lists_for_name: Vec<(String, String)> = data.pinned_lists.iter()
                                .chain(data.recent_lists.iter())
                                .chain(data.root_lists.iter())
                                .map(|l| (l.id.clone(), l.name.clone()))
                                .collect();

                            let del_cb = Callback::new(move |list_id: String| {
                                let name = all_lists_for_name.iter()
                                    .find(|(id, _)| id == &list_id)
                                    .map(|(_, n)| n.clone())
                                    .unwrap_or_default();
                                pending_delete.set(Some((list_id, name)));
                            });

                            view! {
                                <div>
                                    <PinnedSection
                                        pinned_lists=data.pinned_lists.clone()
                                        pinned_containers=data.pinned_containers.clone()
                                        all_tags=tags.clone()
                                        all_links=all_links.clone()
                                        _active_tag_filter=active_tag_filter
                                        on_tag_toggle=on_tag_toggle
                                        on_delete_list=del_cb
                                        on_pin_container=on_pin_container
                                    />
                                    <RecentSection
                                        recent_lists=data.recent_lists.clone()
                                        recent_containers=data.recent_containers.clone()
                                        all_tags=tags.clone()
                                        all_links=all_links.clone()
                                        active_tag_filter=active_tag_filter
                                        on_tag_toggle=on_tag_toggle
                                        on_delete_list=del_cb
                                    />
                                    <RootSection
                                        root_containers=data.root_containers.clone()
                                        root_lists=data.root_lists.clone()
                                        all_tags=tags.clone()
                                        all_links=all_links.clone()
                                        active_tag_filter=active_tag_filter
                                        on_tag_toggle=on_tag_toggle
                                        on_delete_list=del_cb
                                        on_delete_container=on_delete_container
                                        on_pin_container=on_pin_container
                                    />
                                </div>
                            }.into_any()
                        }
                        (Some(Err(e)), _, _) => view! {
                            <p class="text-error">"Error: " {e.to_string()}</p>
                        }.into_any(),
                        _ => view! { <LoadingSpinner/> }.into_any(),
                    }
                }}
            </Transition>

            // Archived section
            <Transition fallback=|| view! {}>
                {move || archived_res.get().map(|result| match result {
                    Ok(archived) if !archived.is_empty() => {
                        let count = archived.len();
                        view! {
                            <div class="collapse collapse-arrow bg-base-200 mt-6">
                                <input type="checkbox" />
                                <div class="collapse-title font-semibold">
                                    "📦 Zarchiwizowane (" {count} ")"
                                </div>
                                <div class="collapse-content">
                                    <div class="flex flex-col gap-2 pt-2">
                                        {archived.into_iter().map(|list| {
                                            let lid = list.id.clone();
                                            view! {
                                                <div class="flex items-center justify-between p-3 bg-base-100 rounded-lg">
                                                    <span class="text-base-content/70">
                                                        {list.name.clone()}
                                                    </span>
                                                    <button
                                                        type="button"
                                                        class="btn btn-ghost btn-sm"
                                                        on:click=move |_| on_restore_list.run(lid.clone())
                                                    >
                                                        "Przywróć"
                                                    </button>
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                </div>
                            </div>
                        }.into_any()
                    }
                    _ => view! {}.into_any(),
                })}
            </Transition>
        </div>
    }
}
