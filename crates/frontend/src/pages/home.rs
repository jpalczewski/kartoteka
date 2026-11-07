use std::collections::HashSet;

use kartoteka_shared::ChangeTopic;
use kartoteka_shared::tag_utils::build_ancestor_map;
use kartoteka_shared::types::{CreateContainerRequest, CreateListRequest};
use kartoteka_shared::{FilterMode, HomeFilterResult};
use leptos::prelude::*;
use leptos_fluent::{I18n, move_tr};

use crate::app::{ToastContext, ToastKind};
use crate::components::common::delete_entity_modal::{
    DeleteEntityKind, DeleteEntityModal, DeleteTarget,
};
use crate::components::common::loading::LoadingSpinner;
use crate::components::home::{
    pinned_section::PinnedSection, recent_section::RecentSection, root_section::RootSection,
    tag_filter_bar::HomeTagFilterBar,
};
use crate::components::lists::create_entity_input::CreateEntityInput;
use crate::pages::landing::LandingPage;
use crate::server_fns::auth::get_auth_status;
use crate::server_fns::{
    containers::{archive_container, create_container, toggle_container_pin},
    home::{get_archived_containers, get_archived_lists, get_home_data},
    lists::{archive_list, create_list},
    tags::{
        assign_tag_to_list, filter_home_by_tags, get_all_tags, get_list_tag_links,
        remove_tag_from_list,
    },
};
use crate::state::invalidation::{mutate, mutate_with, use_invalidation};

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
    let invalidation = use_invalidation();

    // Multi-tag AND filter
    let active_tags: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());
    let filter_mode: RwSignal<FilterMode> = RwSignal::new(FilterMode::default());

    // The entity a delete was requested for; the shared modal fetches its preview.
    let delete_target: RwSignal<Option<DeleteTarget>> = RwSignal::new(None);

    // Resources — fetched server-side at initial render, refetched when a topic they depend on is invalidated
    let home_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::Lists, ChangeTopic::Containers]),
        |_| get_home_data(),
    );
    let archived_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::Lists]),
        |_| get_archived_lists(),
    );
    let archived_containers_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::Containers]),
        |_| get_archived_containers(),
    );
    let tags_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::Tags]),
        |_| get_all_tags(),
    );
    let tag_links_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::Lists, ChangeTopic::Tags]),
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
            let filter = if tags.is_empty() {
                None
            } else {
                Some((tags, filter_mode.get()))
            };
            (
                filter,
                invalidation.track(&[ChangeTopic::Lists, ChangeTopic::Tags]),
            )
        },
        |(filter, _)| async move {
            match filter {
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
        mutate(
            create_list(req),
            &[ChangeTopic::Lists, ChangeTopic::Containers],
        );
    });

    let on_create_container = Callback::new(move |req: CreateContainerRequest| {
        mutate(create_container(req), &[ChangeTopic::Containers]);
    });

    let on_deleted = Callback::new(move |target: DeleteTarget| {
        let msg = i18n.tr(match target.kind {
            DeleteEntityKind::List => "home-list-deleted",
            DeleteEntityKind::Container => "home-container-deleted",
        });
        toast.push(msg, ToastKind::Success);
    });

    let on_pin_container = Callback::new(move |container_id: String| {
        mutate(
            toggle_container_pin(container_id),
            &[ChangeTopic::Containers],
        );
    });

    let on_tag_toggle = Callback::new(move |(list_id, tag_id): (String, String)| {
        let has_tag = tag_links_res
            .get_untracked()
            .and_then(|r| r.ok())
            .unwrap_or_default()
            .iter()
            .any(|l| l.list_id == list_id && l.tag_id == tag_id);
        let result = async move {
            if has_tag {
                remove_tag_from_list(list_id, tag_id).await
            } else {
                assign_tag_to_list(list_id, tag_id).await
            }
        };
        mutate(result, &[ChangeTopic::Lists, ChangeTopic::Tags]);
    });

    let on_restore_list = Callback::new(move |list_id: String| {
        let msg = i18n.tr("home-list-restored");
        mutate_with(
            archive_list(list_id),
            &[ChangeTopic::Lists, ChangeTopic::Containers],
            move |_| toast.push(msg, ToastKind::Success),
        );
    });

    let on_restore_container = {
        let msg_restored = i18n.tr("home-container-restored");
        Callback::new(move |container_id: String| {
            let msg = msg_restored.clone();
            mutate_with(
                archive_container(container_id),
                &[ChangeTopic::Containers, ChangeTopic::Lists],
                move |()| toast.push(msg, ToastKind::Success),
            );
        })
    };

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-4">{move_tr!("home-heading")}</h2>

            <DeleteEntityModal target=delete_target on_deleted=on_deleted />

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
                            // Names for the delete dialog: sections only hand back ids.
                            let list_names: std::collections::HashMap<String, String> = data.pinned_lists.iter()
                                .chain(data.recent_lists.iter())
                                .chain(data.root_lists.iter())
                                .map(|l| (l.id.clone(), l.name.clone()))
                                .collect();
                            let container_names: std::collections::HashMap<String, String> = data.pinned_containers.iter()
                                .chain(data.recent_containers.iter())
                                .chain(data.root_containers.iter())
                                .map(|c| (c.id.clone(), c.name.clone()))
                                .collect();

                            let del_cb = Callback::new(move |list_id: String| {
                                let name = list_names.get(&list_id).cloned().unwrap_or_default();
                                delete_target.set(Some(DeleteTarget::list(list_id, name)));
                            });
                            let del_container_cb = Callback::new(move |container_id: String| {
                                let name = container_names.get(&container_id).cloned().unwrap_or_default();
                                delete_target.set(Some(DeleteTarget::container(container_id, name)));
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
                                        on_delete_container=del_container_cb
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
