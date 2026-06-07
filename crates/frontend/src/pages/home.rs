use std::collections::HashSet;

use kartoteka_shared::tag_utils::build_ancestor_map;
use kartoteka_shared::types::{CreateContainerRequest, CreateListRequest};
use kartoteka_shared::{FilterMode, HomeFilterResult};
use leptos::prelude::*;
use leptos_fluent::{I18n, move_tr};

use crate::app::{ToastContext, ToastKind};
use crate::components::common::{
    confirm_modal::{ConfirmModal, ConfirmVariant},
    loading::LoadingSpinner,
};
use crate::components::home::{
    pinned_section::PinnedSection, recent_section::RecentSection, root_section::RootSection,
    tag_filter_bar::HomeTagFilterBar,
};
use crate::components::lists::create_entity_input::CreateEntityInput;
use crate::context::GlobalRefresh;
use crate::pages::landing::LandingPage;
use crate::server_fns::auth::get_auth_status;
use crate::server_fns::{
    containers::{archive_container, create_container, delete_container, toggle_container_pin},
    home::{get_archived_containers, get_archived_lists, get_home_data},
    lists::{archive_list, create_list, delete_list},
    tags::{
        assign_tag_to_list, filter_home_by_tags, get_all_tags, get_list_tag_links,
        remove_tag_from_list,
    },
};

#[component]
pub fn HomePage() -> impl IntoView {
    let auth = Resource::new(|| (), |_| get_auth_status());
    view! {
        <Suspense fallback=|| view! { <LoadingSpinner/> }>
            {move || auth.get().map(|r| match r {
                Ok(true) => view! { <HomeContent/> }.into_any(),
                Ok(false) => view! { <LandingPage/> }.into_any(),
                Err(_) => view! {
                    <div class="flex min-h-[70vh] items-center justify-center">
                        <p class="text-base-content/70">"Błąd połączenia. "<a href="/" class="link">"Odśwież stronę."</a></p>
                    </div>
                }.into_any(),
            })}
        </Suspense>
    }
}

#[component]
fn HomeContent() -> impl IntoView {
    let i18n = expect_context::<I18n>();
    let toast = use_context::<ToastContext>().expect("ToastContext missing");

    // Refresh trigger — incrementing causes all Resources to refetch
    let (refresh, set_refresh) = signal(0u32);
    let global_refresh = use_context::<GlobalRefresh>().expect("GlobalRefresh missing");

    // Multi-tag AND filter
    let active_tags: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());
    let filter_mode: RwSignal<FilterMode> = RwSignal::new(FilterMode::default());

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
    let archived_containers_res = Resource::new(
        move || (refresh.get(), global_refresh.get()),
        |_| get_archived_containers(),
    );
    let tags_res = Resource::new(|| (), |_| get_all_tags());
    let tag_links_res = Resource::new(
        move || (refresh.get(), global_refresh.get()),
        |_| get_list_tag_links(),
    );

    // Precomputed once when tags load; stable across filter changes
    let ancestor_map = Memo::new(move |_| {
        tags_res
            .get()
            .and_then(|r| r.ok())
            .map(|tags| build_ancestor_map(&tags, "\\"))
            .unwrap_or_default()
    });

    let filter_res = Resource::new(
        move || {
            let mut tags: Vec<String> = active_tags.get().into_iter().collect();
            tags.sort();
            if tags.is_empty() {
                None
            } else {
                Some((tags, filter_mode.get()))
            }
        },
        |key| async move {
            match key {
                None => Ok(HomeFilterResult::default()),
                Some((tags, mode)) => filter_home_by_tags(tags, mode).await,
            }
        },
    );

    let matching_list_ids = Signal::derive(move || {
        if active_tags.get().is_empty() {
            None
        } else {
            filter_res
                .get()
                .and_then(|r| r.ok())
                .map(|r| r.matching_list_ids.into_iter().collect::<HashSet<_>>())
        }
    });

    let matching_container_ids = Signal::derive(move || {
        if active_tags.get().is_empty() {
            None
        } else {
            filter_res
                .get()
                .and_then(|r| r.ok())
                .map(|r| r.matching_container_ids.into_iter().collect::<HashSet<_>>())
        }
    });

    let related_tag_ids = Signal::derive(move || {
        filter_res
            .get()
            .and_then(|r| r.ok())
            .map(|r| r.related_tag_ids.into_iter().collect::<HashSet<_>>())
            .unwrap_or_default()
    });

    let filter_loading =
        Signal::derive(move || !active_tags.get().is_empty() && filter_res.get().is_none());

    let all_tags_signal =
        Signal::derive(move || tags_res.get().and_then(|r| r.ok()).unwrap_or_default());

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
        let msg = i18n.tr("home-list-deleted");
        leptos::task::spawn_local(async move {
            match delete_list(list_id).await {
                Ok(_) => {
                    pending_delete.set(None);
                    set_refresh.update(|n| *n += 1);
                    toast.push(msg, ToastKind::Success);
                }
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_delete_container = Callback::new(move |container_id: String| {
        let msg = i18n.tr("home-container-deleted");
        leptos::task::spawn_local(async move {
            match delete_container(container_id).await {
                Ok(_) => {
                    set_refresh.update(|n| *n += 1);
                    toast.push(msg, ToastKind::Success);
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
                .get_untracked()
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
        let msg = i18n.tr("home-list-restored");
        leptos::task::spawn_local(async move {
            match archive_list(list_id).await {
                Ok(_) => {
                    set_refresh.update(|n| *n += 1);
                    toast.push(msg, ToastKind::Success);
                }
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_restore_container = {
        let msg_restored = i18n.tr("home-container-restored");
        Callback::new(move |container_id: String| {
            let msg = msg_restored.clone();
            leptos::task::spawn_local(async move {
                match archive_container(container_id).await {
                    Ok(_) => {
                        set_refresh.update(|n| *n += 1);
                        toast.push(msg, ToastKind::Success);
                    }
                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                }
            });
        })
    };

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-4">{move_tr!("home-heading")}</h2>

            {move || pending_delete.get().map(|(lid, lname)| {
                let lid_confirm = lid.clone();
                view! {
                    <ConfirmModal
                        open=Signal::derive(move || pending_delete.get().is_some())
                        title=i18n.tr("home-delete-list-title")
                        message=i18n.tr("home-delete-list-confirm").replace("{ $name }", &lname)
                        confirm_label=i18n.tr("common-delete")
                        variant=ConfirmVariant::Danger
                        on_close=Callback::new(move |_| pending_delete.set(None))
                        on_confirm=Callback::new(move |_| {
                            pending_delete.set(None);
                            on_delete_list_confirmed.run(lid_confirm.clone());
                        })
                    />
                }
            })}

            // Tag filter bar — inside Transition so tags_res is serialized for hydration
            <Transition fallback=|| view! {}>
                <HomeTagFilterBar
                    all_tags=all_tags_signal
                    ancestor_map=ancestor_map
                    active_tags=active_tags
                    filter_mode=filter_mode
                    related_tag_ids=related_tag_ids
                    is_loading=filter_loading
                />
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
                    let _matching = matching_list_ids.get();
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
                                        matching_list_ids=matching_list_ids
                                        matching_container_ids=matching_container_ids
                                        on_tag_toggle=on_tag_toggle
                                        on_delete_list=del_cb
                                        on_pin_container=on_pin_container
                                    />
                                    <RecentSection
                                        recent_lists=data.recent_lists.clone()
                                        recent_containers=data.recent_containers.clone()
                                        all_tags=tags.clone()
                                        all_links=all_links.clone()
                                        matching_list_ids=matching_list_ids
                                        matching_container_ids=matching_container_ids
                                        on_tag_toggle=on_tag_toggle
                                        on_delete_list=del_cb
                                    />
                                    <RootSection
                                        root_containers=data.root_containers.clone()
                                        root_lists=data.root_lists.clone()
                                        all_tags=tags.clone()
                                        all_links=all_links.clone()
                                        matching_list_ids=matching_list_ids
                                        matching_container_ids=matching_container_ids
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
                {move || {
                    let lists = archived_res.get().and_then(|r| r.ok()).unwrap_or_default();
                    let containers = archived_containers_res.get().and_then(|r| r.ok()).unwrap_or_default();
                    let total = lists.len() + containers.len();
                    if total == 0 {
                        return view! {}.into_any();
                    }
                    view! {
                        <div class="collapse collapse-arrow bg-base-200 mt-6">
                            <input type="checkbox" />
                            <div class="collapse-title font-semibold">
                                {move_tr!("home-archive", { "count" => total })}
                            </div>
                            <div class="collapse-content">
                                <div class="flex flex-col gap-2 pt-2">
                                    {containers.into_iter().map(|c| {
                                        let cid = c.id.clone();
                                        view! {
                                            <div class="flex items-center justify-between p-3 bg-base-100 rounded-lg">
                                                <span class="text-base-content/70">{"📁 "}{c.name.clone()}</span>
                                                <button
                                                    type="button"
                                                    class="btn btn-ghost btn-sm"
                                                    on:click=move |_| on_restore_container.run(cid.clone())
                                                >
                                                    {move_tr!("home-restore-button")}
                                                </button>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                    {lists.into_iter().map(|list| {
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
                                                    {move_tr!("home-restore-button")}
                                                </button>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        </div>
                    }.into_any()
                }}
            </Transition>
        </div>
    }
}
