mod date_view;
mod normal_view;

use std::collections::HashSet;

use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::api;
use crate::api::client::GlooClient;
use crate::app::{ToastContext, ToastKind};
use crate::components::common::breadcrumbs::{
    BreadcrumbCrumb, Breadcrumbs, build_container_breadcrumbs, build_list_ancestor_breadcrumbs,
};
use crate::components::common::dnd::{
    DragGrip, END_DROP_TARGET_ID, drag_handle_class, drag_shell_class, drag_surface_class,
    drop_marker_class, drop_marker_label_class, drop_marker_line_class,
};
use crate::components::common::editable_description::EditableDescription;
use crate::components::common::loading::LoadingSpinner;
use crate::components::items::add_item_input::AddItemInput;
use crate::components::items::item_actions::create_item_actions;
use crate::components::lists::add_group_input::AddGroupInput;
use crate::components::lists::list_header::ListHeader;
use crate::components::lists::list_tag_bar::ListTagBar;
use crate::components::lists::sublist_section::SublistSection;
use crate::components::tags::tag_filter_bar::TagFilterBar;
use crate::components::tags::tag_tree::build_tag_filter_options;
use crate::state::item_mutations::run_optimistic_mutation;
use crate::state::reorder::{apply_reorder, reorder_ids};
use kartoteka_shared::{
    FEATURE_DEADLINES, FEATURE_QUANTITY, Item, ItemTagLink, List, ListFeature, ListTagLink,
    ReorderItemsRequest, SetListPlacementRequest, Tag, UpdateListRequest,
};

use date_view::render_date_view;
use normal_view::{NormalViewProps, render_normal_view};

async fn fetch_list_ancestors(client: &GlooClient, list: &List) -> Vec<List> {
    let mut ancestors = Vec::new();
    let mut current_parent_id = list.parent_list_id.clone();
    let mut depth = 0;

    while let Some(parent_id) = current_parent_id {
        if depth > 10 {
            break;
        }

        match api::fetch_list(client, &parent_id).await {
            Ok(parent) => {
                current_parent_id = parent.parent_list_id.clone();
                ancestors.push(parent);
            }
            Err(_) => break,
        }
        depth += 1;
    }

    ancestors.reverse();
    ancestors
}

async fn resolve_list_breadcrumbs(client: &GlooClient, list: &List) -> Vec<BreadcrumbCrumb> {
    let ancestors = fetch_list_ancestors(client, list).await;
    let mut crumbs = Vec::new();

    let root_container_id = ancestors
        .first()
        .and_then(|ancestor| ancestor.container_id.clone())
        .or_else(|| list.container_id.clone());

    if let Some(container_id) = root_container_id
        && let Ok(all_containers) = api::fetch_containers(client).await
    {
        crumbs.extend(build_container_breadcrumbs(
            &container_id,
            &all_containers,
            true,
        ));
    }

    crumbs.extend(build_list_ancestor_breadcrumbs(&ancestors));
    crumbs
}

#[component]
pub fn ListPage() -> impl IntoView {
    let params = use_params_map();
    let list_id = move || params.get_untracked().get("id").unwrap_or_default();

    let client = use_context::<GlooClient>().expect("GlooClient not provided");

    let items = RwSignal::new(Vec::<Item>::new());
    let all_tags = RwSignal::new(Vec::<Tag>::new());
    let item_tag_links = RwSignal::new(Vec::<ItemTagLink>::new());
    let list_tag_links = RwSignal::new(Vec::<ListTagLink>::new());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(Option::<String>::None);
    let breadcrumbs = RwSignal::new(Vec::<BreadcrumbCrumb>::new());

    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let navigate = use_navigate();
    let list_name = RwSignal::new(String::new());
    let list_description = RwSignal::new(Option::<String>::None);
    let list_features = RwSignal::new(Vec::<ListFeature>::new());
    let sublists = RwSignal::new(Vec::<List>::new());
    let dragged_item_id = RwSignal::new(Option::<String>::None);
    let dragged_sublist_id = RwSignal::new(Option::<String>::None);
    let hovered_sublist_drop_id = RwSignal::new(Option::<String>::None);
    let is_end_sublist_drop_hovered = Signal::derive(move || {
        hovered_sublist_drop_id.get().as_deref() == Some(END_DROP_TARGET_ID)
    });

    let lid = list_id();
    let client_init = client.clone();
    leptos::task::spawn_local(async move {
        if let Ok(list) = api::fetch_list(&client_init, &lid).await {
            breadcrumbs.set(resolve_list_breadcrumbs(&client_init, &list).await);
            list_name.set(list.name);
            list_description.set(list.description);
            list_features.set(list.features);
        }
        match api::fetch_items(&client_init, &lid).await {
            Ok(fetched) => items.set(fetched),
            Err(e) => set_error.set(Some(e.to_string())),
        }
        if let Ok(fetched_tags) = api::fetch_tags(&client_init).await {
            all_tags.set(fetched_tags);
        }
        if let Ok(links) = api::fetch_item_tag_links(&client_init).await {
            item_tag_links.set(links);
        }
        if let Ok(fetched_sublists) = api::fetch_sublists(&client_init, &lid).await {
            sublists.set(fetched_sublists);
        }
        if let Ok(links) = api::fetch_list_tag_links(&client_init).await {
            let filtered: Vec<ListTagLink> =
                links.into_iter().filter(|l| l.list_id == lid).collect();
            list_tag_links.set(filtered);
        }
        set_loading.set(false);
    });

    let actions = create_item_actions(client.clone(), items, list_id(), Some(set_error));
    let on_add = actions.on_add;
    let on_toggle = actions.on_toggle;
    let on_delete = actions.on_delete;
    let on_description_save = actions.on_description_save;
    let on_quantity_change = actions.on_quantity_change;
    let on_date_save = actions.on_date_save;

    let on_tag_toggle = make_item_tag_toggle(client.clone(), item_tag_links);
    let lid_for_tag = list_id();
    let on_list_tag_toggle = make_list_tag_toggle(client.clone(), list_tag_links, lid_for_tag);
    let on_move_main = make_move_callback(client.clone(), items);

    let parent_lid = list_id();
    let on_item_moved_out = Callback::new(move |(moved_item, target_list_id): (Item, String)| {
        if target_list_id == parent_lid {
            items.update(|list| list.push(moved_item));
        }
    });

    let on_delete_list = {
        let navigate = navigate.clone();
        let client = client.clone();
        Callback::new(move |_: ()| {
            let lid = list_id();
            let nav = navigate.clone();
            let client = client.clone();
            leptos::task::spawn_local(async move {
                match api::delete_list(&client, &lid).await {
                    Ok(()) => {
                        toast.push("Lista usunięta".into(), ToastKind::Success);
                        nav("/", Default::default());
                    }
                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                }
            });
        })
    };

    let on_archive = {
        let navigate = use_navigate();
        let client = client.clone();
        Callback::new(move |_: ()| {
            let lid = list_id();
            let nav = navigate.clone();
            let client = client.clone();
            leptos::task::spawn_local(async move {
                match api::archive_list(&client, &lid).await {
                    Ok(_) => {
                        toast.push("Lista zarchiwizowana".into(), ToastKind::Success);
                        nav("/", Default::default());
                    }
                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                }
            });
        })
    };

    let on_reset = {
        let client = client.clone();
        Callback::new(move |_: ()| {
            let lid = list_id();
            let client = client.clone();
            leptos::task::spawn_local(async move {
                match api::reset_list(&client, &lid).await {
                    Ok(()) => {
                        items.update(|list| {
                            for item in list.iter_mut() {
                                item.completed = false;
                                item.actual_quantity = Some(0);
                            }
                        });
                        toast.push("Lista zresetowana".into(), ToastKind::Success);
                    }
                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                }
            });
        })
    };

    let on_create_group = {
        let client = client.clone();
        Callback::new(move |name: String| {
            let lid = list_id();
            let client = client.clone();
            leptos::task::spawn_local(async move {
                if let Ok(sl) = api::create_sublist(&client, &lid, &name).await {
                    sublists.update(|list| list.push(sl));
                }
            });
        })
    };

    let on_main_item_drop = {
        let client = client.clone();
        Callback::new(move |before_id: Option<String>| {
            let Some(dragged_id) = dragged_item_id.get_untracked() else {
                return;
            };
            let current_ids: Vec<String> = items
                .get_untracked()
                .into_iter()
                .map(|item| item.id)
                .collect();
            let Some(next_ids) = reorder_ids(&current_ids, &dragged_id, before_id.as_deref())
            else {
                dragged_item_id.set(None);
                return;
            };

            let lid = list_id();
            let request = ReorderItemsRequest {
                item_ids: next_ids.clone(),
            };
            let dragged_id_for_mutation = dragged_id.clone();
            let before_id_for_mutation = before_id.clone();
            let client = client.clone();
            run_optimistic_mutation(
                items,
                move |items| {
                    let current_ids: Vec<String> =
                        items.iter().map(|item| item.id.clone()).collect();
                    let Some(next_ids) = reorder_ids(
                        &current_ids,
                        &dragged_id_for_mutation,
                        before_id_for_mutation.as_deref(),
                    ) else {
                        return false;
                    };
                    apply_reorder(items, &next_ids, |item| item.id.as_str())
                },
                move || async move { api::reorder_items(&client, &lid, &request).await },
                move |error| set_error.set(Some(error.to_string())),
            );
            dragged_item_id.set(None);
        })
    };

    let on_sublist_drop = {
        let client = client.clone();
        Callback::new(move |before_id: Option<String>| {
            let Some(dragged_id) = dragged_sublist_id.get_untracked() else {
                return;
            };
            let current_ids: Vec<String> = sublists
                .get_untracked()
                .into_iter()
                .map(|list| list.id)
                .collect();
            let Some(next_ids) = reorder_ids(&current_ids, &dragged_id, before_id.as_deref())
            else {
                dragged_sublist_id.set(None);
                return;
            };

            let request = SetListPlacementRequest {
                list_ids: next_ids.clone(),
                parent_list_id: Some(list_id()),
                container_id: None,
            };
            let dragged_id_for_mutation = dragged_id.clone();
            let before_id_for_mutation = before_id.clone();
            let client = client.clone();
            run_optimistic_mutation(
                sublists,
                move |lists| {
                    let current_ids: Vec<String> =
                        lists.iter().map(|list| list.id.clone()).collect();
                    let Some(next_ids) = reorder_ids(
                        &current_ids,
                        &dragged_id_for_mutation,
                        before_id_for_mutation.as_deref(),
                    ) else {
                        return false;
                    };
                    apply_reorder(lists, &next_ids, |list| list.id.as_str())
                },
                move || async move { api::reorder_lists(&client, &request).await },
                move |error| toast.push(error.to_string(), ToastKind::Error),
            );
            dragged_sublist_id.set(None);
        })
    };

    let (active_tag_filter, set_active_tag_filter) = signal(Option::<String>::None);

    let sorted_items = move || {
        let mut list = items.get();
        list.sort_by(|a, b| a.position.cmp(&b.position));
        list
    };

    let filtered_items = move || {
        let items = sorted_items();
        match active_tag_filter.get() {
            None => items,
            Some(tid) => {
                let tagged_item_ids: Vec<String> = item_tag_links
                    .read()
                    .iter()
                    .filter(|l| l.tag_id == tid)
                    .map(|l| l.item_id.clone())
                    .collect();
                items
                    .into_iter()
                    .filter(|i| tagged_item_ids.contains(&i.id))
                    .collect()
            }
        }
    };

    let tag_filter_options = move || {
        let tags = all_tags.get();
        let item_ids: HashSet<String> = items.get().into_iter().map(|item| item.id).collect();
        let relevant_tag_ids: Vec<String> = item_tag_links
            .get()
            .into_iter()
            .filter(|link| item_ids.contains(&link.item_id))
            .map(|link| link.tag_id)
            .collect();
        build_tag_filter_options(&tags, &relevant_tag_ids)
    };

    /// Get deadlines feature config from list features, or Null if not enabled
    fn get_deadlines_config(feats: &[ListFeature]) -> serde_json::Value {
        feats
            .iter()
            .find(|f| f.name == FEATURE_DEADLINES)
            .map(|f| f.config.clone())
            .unwrap_or(serde_json::Value::Null)
    }

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
                let client = use_context::<GlooClient>().expect("GlooClient not provided");
                view! {
                <ListHeader
                    list_name=list_name.get()
                    list_id=list_id()
                    item_count=items.read().len()
                    on_delete_confirmed=on_delete_list
                    on_archive=on_archive
                    on_reset=on_reset
                    features=list_features.get()
                    on_feature_toggle=Callback::new(move |(feature_name, enabled): (String, bool)| {
                        let lid = list_id();
                        let client = client.clone();
                        // Optimistic update
                        list_features.update(|feats| {
                            if enabled {
                                if !feats.iter().any(|f| f.name == feature_name) {
                                    let config = if feature_name == FEATURE_DEADLINES {
                                        serde_json::json!({"has_start_date": false, "has_deadline": true, "has_hard_deadline": false})
                                    } else {
                                        serde_json::json!({})
                                    };
                                    feats.push(ListFeature {
                                        name: feature_name.clone(),
                                        config,
                                    });
                                }
                            } else {
                                feats.retain(|f| f.name != feature_name);
                            }
                        });
                        let fname = feature_name.clone();
                        let config = if fname == FEATURE_DEADLINES {
                            serde_json::json!({"has_start_date": false, "has_deadline": true, "has_hard_deadline": false})
                        } else {
                            serde_json::json!({})
                        };
                        leptos::task::spawn_local(async move {
                            if enabled {
                                let _ = api::add_feature(&client, &lid, &fname, config).await;
                            } else {
                                let _ = api::remove_feature(&client, &lid, &fname).await;
                            }
                        });
                    })
                    on_deadlines_config_change=Callback::new(move |new_config: serde_json::Value| {
                        let lid = list_id();
                        let client = use_context::<GlooClient>().expect("GlooClient not provided");
                        // Optimistic update
                        list_features.update(|feats| {
                            if let Some(f) = feats.iter_mut().find(|f| f.name == FEATURE_DEADLINES) {
                                f.config = new_config.clone();
                            }
                        });
                        leptos::task::spawn_local(async move {
                            let _ = api::add_feature(&client, &lid, FEATURE_DEADLINES, new_config).await;
                        });
                    })
                    on_rename=Callback::new(move |new_name: String| {
                        list_name.set(new_name.clone());
                        let lid = list_id();
                        let client = use_context::<GlooClient>().expect("GlooClient not provided");
                        leptos::task::spawn_local(async move {
                            let req = UpdateListRequest {
                                name: Some(new_name),
                                description: None,
                                list_type: None,
                                archived: None,
                            };
                            let _ = api::update_list(&client, &lid, &req).await;
                        });
                    })
                />
            }}}

            {move || {
                view! {
                <EditableDescription
                    value=list_description.get()
                    on_save=Callback::new(move |new_desc: Option<String>| {
                        list_description.set(new_desc.clone());
                        let lid = list_id();
                        let client = use_context::<GlooClient>().expect("GlooClient not provided");
                        leptos::task::spawn_local(async move {
                            let req = UpdateListRequest {
                                name: None,
                                description: Some(new_desc),
                                list_type: None,
                                archived: None,
                            };
                            let _ = api::update_list(&client, &lid, &req).await;
                        });
                    })
                />
            }}}

            {move || {
                let links = list_tag_links.read();
                let tags = all_tags.read();
                let assigned_ids: Vec<String> = links.iter().map(|l| l.tag_id.clone()).collect();
                view! {
                    <ListTagBar
                        all_tags=tags.clone()
                        assigned_tag_ids=assigned_ids
                        on_toggle=on_list_tag_toggle
                    />
                }
            }}

            {move || {
                let feats = list_features.get();
                let deadlines_config = get_deadlines_config(&feats);
                view! { <AddItemInput on_submit=on_add has_quantity=feats.iter().any(|f| f.name == FEATURE_QUANTITY) deadlines_config=deadlines_config /> }
            }}

            {move || {
                view! {
                    <TagFilterBar
                        tags=tag_filter_options()
                        active_tag_id=active_tag_filter
                        on_select=set_active_tag_filter
                    />
                }
            }}

            {move || {
                if loading.get() {
                    view! { <LoadingSpinner/> }.into_any()
                } else if let Some(e) = error.get() {
                    view! { <p style="color: red;">{format!("B\u{0142}\u{0105}d: {e}")}</p> }.into_any()
                } else if items.read().is_empty() && sublists.read().is_empty() {
                    view! { <div class="text-center text-base-content/50 py-12">"Lista jest pusta"</div> }.into_any()
                } else {
                    view! {
                        <div>
                            {move || {
                                let feats = list_features.get();
                                if feats.iter().any(|f| f.name == FEATURE_DEADLINES) {
                                    render_date_view(filtered_items(), all_tags.get(), item_tag_links.get(), on_toggle, on_delete, on_tag_toggle, on_date_save).into_any()
                                } else {
                                    render_normal_view(NormalViewProps {
                                        items: filtered_items(),
                                        tags: all_tags.get(),
                                        item_tag_links,
                                        sublists: sublists.get(),
                                        on_toggle, on_delete, on_tag_toggle,
                                        on_description_save, on_quantity_change,
                                        has_quantity: feats.iter().any(|f| f.name == FEATURE_QUANTITY),
                                        on_move: on_move_main,
                                        on_date_save,
                                        deadlines_config: get_deadlines_config(&feats),
                                        enable_reorder: active_tag_filter.get().is_none(),
                                        dragged_item_id,
                                        on_reorder_drop: on_main_item_drop,
                                    }).into_any()
                                }
                            }}

                            {move || {
                                let subs = sublists.get();
                                if subs.is_empty() {
                                    ().into_any()
                                } else {
                                    view! {
                                        <div class="mt-6">
                                            {subs.iter().map(|sl| {
                                                let drag_id = sl.id.clone();
                                                let drop_before_id = sl.id.clone();
                                                let drop_target_id = sl.id.clone();
                                                let drop_target_id_for_dragover = drop_target_id.clone();
                                                let drop_before_id_for_drop = drop_before_id.clone();
                                                let drag_id_for_drag = drag_id.clone();
                                                let drag_id_for_shell = drag_id.clone();
                                                let drag_id_for_surface = drag_id.clone();
                                                let drop_target_id_for_hover = drop_target_id.clone();
                                                let is_drop_target_hovered = Signal::derive(move || {
                                                    hovered_sublist_drop_id.get().as_deref() == Some(drop_target_id_for_hover.as_str())
                                                });
                                                let tags = all_tags.get();
                                                let links = item_tag_links.get();
                                                let lid = list_id();
                                                let lname = list_name.get();
                                                let sl_id = sl.id.clone();
                                                let mut mt: Vec<(String, String)> = vec![
                                                    (lid, format!("{lname} (g\u{0142}\u{00F3}wna)"))
                                                ];
                                                mt.extend(
                                                    subs.iter()
                                                        .filter(|s| s.id != sl_id)
                                                        .map(|s| (s.id.clone(), s.name.clone()))
                                                );
                                                let feats = list_features.get();
                                                let deadlines_config = get_deadlines_config(&feats);
                                                view! {
                                                    <div class="flex flex-col gap-2">
                                                        <div
                                                            class=move || drop_marker_class(
                                                                dragged_sublist_id.get().is_some(),
                                                                is_drop_target_hovered.get(),
                                                            )
                                                            on:dragover=move |ev: web_sys::DragEvent| {
                                                                ev.prevent_default();
                                                                if let Some(data_transfer) = ev.data_transfer() {
                                                                    data_transfer.set_drop_effect("move");
                                                                }
                                                                hovered_sublist_drop_id.set(Some(drop_target_id_for_dragover.clone()));
                                                            }
                                                            on:drop=move |ev: web_sys::DragEvent| {
                                                                ev.prevent_default();
                                                                hovered_sublist_drop_id.set(None);
                                                                on_sublist_drop.run(Some(drop_before_id_for_drop.clone()));
                                                            }
                                                        >
                                                            <span class=move || drop_marker_line_class(
                                                                dragged_sublist_id.get().is_some(),
                                                                is_drop_target_hovered.get(),
                                                            )></span>
                                                            <span class=move || drop_marker_label_class(
                                                                dragged_sublist_id.get().is_some(),
                                                                is_drop_target_hovered.get(),
                                                            )>"Upuść tutaj"</span>
                                                            <span class=move || drop_marker_line_class(
                                                                dragged_sublist_id.get().is_some(),
                                                                is_drop_target_hovered.get(),
                                                            )></span>
                                                        </div>
                                                        <div class=move || drag_shell_class(
                                                            dragged_sublist_id.get().as_deref() == Some(drag_id_for_shell.as_str())
                                                        )>
                                                            <button
                                                                type="button"
                                                                class=move || drag_handle_class(
                                                                    dragged_sublist_id.get().as_deref() == Some(drag_id.as_str())
                                                                )
                                                                draggable="true"
                                                                aria-label="Przeciągnij, aby zmienić kolejność grupy"
                                                                title="Przeciągnij, aby zmienić kolejność grupy"
                                                                on:dragstart=move |ev: web_sys::DragEvent| {
                                                                    if let Some(data_transfer) = ev.data_transfer() {
                                                                        let _ = data_transfer.set_data("text/plain", &drag_id_for_drag);
                                                                        data_transfer.set_effect_allowed("move");
                                                                    }
                                                                    dragged_sublist_id.set(Some(drag_id_for_drag.clone()));
                                                                }
                                                                on:dragend=move |_| {
                                                                    dragged_sublist_id.set(None);
                                                                    hovered_sublist_drop_id.set(None);
                                                                }
                                                            >
                                                                <DragGrip />
                                                            </button>
                                                            <div class=move || drag_surface_class(
                                                                dragged_sublist_id.get().as_deref() == Some(drag_id_for_surface.as_str()),
                                                                is_drop_target_hovered.get(),
                                                            )>
                                                                <SublistSection
                                                                    sublist=sl.clone()
                                                                    has_quantity=feats.iter().any(|f| f.name == FEATURE_QUANTITY)
                                                                    deadlines_config=deadlines_config
                                                                    all_tags=tags
                                                                    item_tag_links=links
                                                                    on_tag_toggle=on_tag_toggle
                                                                    move_targets=mt
                                                                    on_item_moved_out=on_item_moved_out
                                                                />
                                                            </div>
                                                        </div>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                            <div
                                                class=move || drop_marker_class(
                                                    dragged_sublist_id.get().is_some(),
                                                    is_end_sublist_drop_hovered.get(),
                                                )
                                                on:dragover=move |ev: web_sys::DragEvent| {
                                                    ev.prevent_default();
                                                    if let Some(data_transfer) = ev.data_transfer() {
                                                        data_transfer.set_drop_effect("move");
                                                    }
                                                    hovered_sublist_drop_id.set(Some(END_DROP_TARGET_ID.to_string()));
                                                }
                                                on:drop=move |ev: web_sys::DragEvent| {
                                                    ev.prevent_default();
                                                    hovered_sublist_drop_id.set(None);
                                                    on_sublist_drop.run(None);
                                                }
                                            >
                                                <span class=move || drop_marker_line_class(
                                                    dragged_sublist_id.get().is_some(),
                                                    is_end_sublist_drop_hovered.get(),
                                                )></span>
                                                <span class=move || drop_marker_label_class(
                                                    dragged_sublist_id.get().is_some(),
                                                    is_end_sublist_drop_hovered.get(),
                                                )>"Upuść na końcu"</span>
                                                <span class=move || drop_marker_line_class(
                                                    dragged_sublist_id.get().is_some(),
                                                    is_end_sublist_drop_hovered.get(),
                                                )></span>
                                            </div>
                                        </div>
                                    }.into_any()
                                }
                            }}
                        </div>
                    }.into_any()
                }
            }}

            <AddGroupInput on_submit=on_create_group />
        </div>
    }
}

fn make_item_tag_toggle(
    client: GlooClient,
    item_tag_links: RwSignal<Vec<ItemTagLink>>,
) -> Callback<(String, String)> {
    Callback::new(move |(item_id, tag_id): (String, String)| {
        let has_tag = item_tag_links
            .read()
            .iter()
            .any(|l| l.item_id == item_id && l.tag_id == tag_id);
        if has_tag {
            item_tag_links
                .update(|links| links.retain(|l| !(l.item_id == item_id && l.tag_id == tag_id)));
            let iid = item_id.clone();
            let tid = tag_id.clone();
            let client = client.clone();
            leptos::task::spawn_local(async move {
                let _ = api::remove_tag_from_item(&client, &iid, &tid).await;
            });
        } else {
            item_tag_links.update(|links| {
                links.push(ItemTagLink {
                    item_id: item_id.clone(),
                    tag_id: tag_id.clone(),
                })
            });
            let iid = item_id.clone();
            let tid = tag_id.clone();
            let client = client.clone();
            leptos::task::spawn_local(async move {
                let _ = api::assign_tag_to_item(&client, &iid, &tid).await;
            });
        }
    })
}

fn make_list_tag_toggle(
    client: GlooClient,
    list_tag_links: RwSignal<Vec<ListTagLink>>,
    list_id: String,
) -> Callback<String> {
    Callback::new(move |tag_id: String| {
        let has_tag = list_tag_links.read().iter().any(|l| l.tag_id == tag_id);
        if has_tag {
            list_tag_links.update(|links| links.retain(|l| l.tag_id != tag_id));
            let lid = list_id.clone();
            let tid = tag_id.clone();
            let client = client.clone();
            leptos::task::spawn_local(async move {
                let _ = api::remove_tag_from_list(&client, &lid, &tid).await;
            });
        } else {
            let lid = list_id.clone();
            list_tag_links.update(|links| {
                links.push(ListTagLink {
                    list_id: lid.clone(),
                    tag_id: tag_id.clone(),
                })
            });
            let tid = tag_id.clone();
            let client = client.clone();
            leptos::task::spawn_local(async move {
                let _ = api::assign_tag_to_list(&client, &lid, &tid).await;
            });
        }
    })
}

fn make_move_callback(
    client: GlooClient,
    items: RwSignal<Vec<Item>>,
) -> Callback<(String, String)> {
    Callback::new(move |(item_id, target_list_id): (String, String)| {
        items.update(|list| list.retain(|i| i.id != item_id));
        let client = client.clone();
        leptos::task::spawn_local(async move {
            let _ = api::move_item(&client, &item_id, &target_list_id).await;
        });
    })
}
