mod date_view;
mod normal_view;

use std::collections::{HashMap, HashSet};

use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::api;
use crate::api::client::GlooClient;
use crate::app::{ToastContext, ToastKind};
use crate::components::common::breadcrumbs::{
    BreadcrumbCrumb, Breadcrumbs, build_container_breadcrumbs, build_list_ancestor_breadcrumbs,
};
use crate::components::common::dnd::{
    DragHandleButton, DragHandleLabel, DragShell, DragSurface, ReorderDropTarget,
};
use crate::components::common::editable_description::EditableDescription;
use crate::components::common::loading::LoadingSpinner;
use crate::components::items::add_item_input::AddItemInput;
use crate::components::lists::add_group_input::AddGroupInput;
use crate::components::lists::list_header::ListHeader;
use crate::components::lists::list_tag_bar::ListTagBar;
use crate::components::lists::sublist_section::SublistSection;
use crate::components::tags::tag_filter_bar::TagFilterBar;
use crate::components::tags::tag_tree::build_tag_filter_options;
use crate::state::dnd::{
    DndState, DraggedItem, DropTarget, ItemDndState, ItemDropPlan, ItemDropTarget,
    build_item_drop_plan, reorder_ids_for_target,
};
use crate::state::item_mutations::{
    ItemDateField, apply_date_change_to_items, build_date_update_request, run_optimistic_mutation,
};
use crate::state::reorder::apply_reorder;
use kartoteka_shared::{
    CreateItemRequest, FEATURE_DEADLINES, FEATURE_QUANTITY, Item, ItemTagLink, List, ListFeature,
    ListTagLink, ReorderItemsRequest, SetListPlacementRequest, Tag, UpdateItemRequest,
    UpdateListRequest,
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
    let sublist_items = RwSignal::new(HashMap::<String, Vec<Item>>::new());
    let item_dnd_state = RwSignal::new(ItemDndState::default());
    let sublist_dnd_state = RwSignal::new(DndState::default());

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
            let mut fetched_sublist_items = HashMap::new();
            for sublist in &fetched_sublists {
                if let Ok(fetched) = api::fetch_items(&client_init, &sublist.id).await {
                    fetched_sublist_items.insert(sublist.id.clone(), fetched);
                }
            }
            sublists.set(fetched_sublists);
            sublist_items.set(fetched_sublist_items);
        }
        if let Ok(links) = api::fetch_list_tag_links(&client_init).await {
            let filtered: Vec<ListTagLink> =
                links.into_iter().filter(|l| l.list_id == lid).collect();
            list_tag_links.set(filtered);
        }
        set_loading.set(false);
    });

    let on_tag_toggle = make_item_tag_toggle(client.clone(), item_tag_links);
    let lid_for_tag = list_id();
    let on_list_tag_toggle = make_list_tag_toggle(client.clone(), list_tag_links, lid_for_tag);
    let main_list_id = list_id();

    let apply_item_drop_plan = {
        let client = client.clone();
        let main_list_id = main_list_id.clone();
        Callback::new(move |plan: ItemDropPlan| {
            let client = client.clone();
            let main_list_id = main_list_id.clone();
            let request_plan = plan.clone();
            run_item_store_mutation(
                items,
                sublist_items,
                move |main_items, sub_items| {
                    apply_item_drop_plan_locally(&main_list_id, main_items, sub_items, &plan)
                },
                move || async move {
                    match request_plan {
                        ItemDropPlan::Reorder { list_id, item_ids } => {
                            api::reorder_items(&client, &list_id, &ReorderItemsRequest { item_ids })
                                .await
                        }
                        ItemDropPlan::Move { item_id, request } => {
                            api::set_item_placement(&client, &item_id, &request)
                                .await
                                .map(|_| ())
                        }
                    }
                },
                move |error| set_error.set(Some(error.to_string())),
            );
        })
    };

    let main_item_callbacks = build_scoped_item_callbacks(
        client.clone(),
        main_list_id.clone(),
        main_list_id.clone(),
        items,
        sublist_items,
        set_error,
        apply_item_drop_plan.clone(),
    );

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
                        toast.push(
                            move_tr!("lists-toast-list-deleted").get(),
                            ToastKind::Success,
                        );
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
                        toast.push(
                            move_tr!("lists-toast-list-archived").get(),
                            ToastKind::Success,
                        );
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
                        toast.push(move_tr!("lists-toast-list-reset").get(), ToastKind::Success);
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
                    let sublist_id = sl.id.clone();
                    sublists.update(|list| list.push(sl));
                    sublist_items.update(|items_by_scope| {
                        items_by_scope.entry(sublist_id).or_default();
                    });
                }
            });
        })
    };

    let on_main_item_drop = {
        let main_list_id = main_list_id.clone();
        let apply_item_drop_plan = apply_item_drop_plan.clone();
        Callback::new(move |target: ItemDropTarget| {
            let Some(dragged_item) = item_dnd_state.get_untracked().dragged_item.clone() else {
                return;
            };
            let item_ids_by_scope = collect_item_ids_by_scope(&main_list_id, items, sublist_items);
            let Some(plan) = build_item_drop_plan(&item_ids_by_scope, &dragged_item, &target)
            else {
                return;
            };
            apply_item_drop_plan.run(plan);
        })
    };

    let on_sublist_drop = {
        let client = client.clone();
        Callback::new(move |target: DropTarget| {
            let Some(dragged_id) = sublist_dnd_state.get_untracked().dragged_id.clone() else {
                return;
            };
            let current_ids: Vec<String> = sublists
                .get_untracked()
                .into_iter()
                .map(|list| list.id)
                .collect();
            let Some(next_ids) = reorder_ids_for_target(&current_ids, &dragged_id, &target) else {
                return;
            };

            let request = SetListPlacementRequest {
                list_ids: next_ids.clone(),
                parent_list_id: Some(list_id()),
                container_id: None,
            };
            let dragged_id_for_mutation = dragged_id.clone();
            let target_for_mutation = target.clone();
            let client = client.clone();
            run_optimistic_mutation(
                sublists,
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

    let (active_tag_filter, set_active_tag_filter) = signal(Option::<String>::None);

    let filtered_items = move || {
        let items = items.get();
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
                view! {
                    <AddItemInput
                        on_submit=main_item_callbacks.on_add
                        has_quantity=feats.iter().any(|f| f.name == FEATURE_QUANTITY)
                        deadlines_config=deadlines_config
                    />
                }
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
                let main_list_id_for_view = main_list_id.clone();
                let main_item_callbacks_for_view = main_item_callbacks.clone();
                let on_main_item_drop_for_view = on_main_item_drop.clone();
                let apply_item_drop_plan_for_view = apply_item_drop_plan.clone();
                let main_list_id_for_main_view = main_list_id_for_view.clone();
                let main_list_id_for_sublist_view = main_list_id_for_view.clone();
                if loading.get() {
                    view! { <LoadingSpinner/> }.into_any()
                } else if let Some(e) = error.get() {
                    let error_detail = e.clone();
                    view! {
                        <p style="color: red;">
                            {move_tr!("lists-inline-error", { "detail" => error_detail.clone() }).get()}
                        </p>
                    }
                    .into_any()
                } else if items.read().is_empty() && sublists.read().is_empty() {
                    view! { <div class="text-center text-base-content/50 py-12">{move_tr!("lists-empty")}</div> }.into_any()
                } else {
                    view! {
                        <div>
                            {move || {
                                let main_list_id_for_main_section = main_list_id_for_main_view.clone();
                                let feats = list_features.get();
                                let enable_item_dnd = active_tag_filter.get().is_none()
                                    && !feats.iter().any(|feature| feature.name == FEATURE_DEADLINES);
                                if feats.iter().any(|f| f.name == FEATURE_DEADLINES) {
                                    render_date_view(
                                        filtered_items(),
                                        all_tags.get(),
                                        item_tag_links.get(),
                                        main_item_callbacks_for_view.on_toggle,
                                        main_item_callbacks_for_view.on_delete,
                                        on_tag_toggle,
                                        main_item_callbacks_for_view.on_date_save,
                                    )
                                    .into_any()
                                } else {
                                    render_normal_view(NormalViewProps {
                                        list_id: main_list_id_for_main_section,
                                        items: filtered_items(),
                                        tags: all_tags.get(),
                                        item_tag_links,
                                        sublists: sublists.get(),
                                        on_toggle: main_item_callbacks_for_view.on_toggle,
                                        on_delete: main_item_callbacks_for_view.on_delete,
                                        on_tag_toggle,
                                        on_description_save: main_item_callbacks_for_view.on_description_save,
                                        on_quantity_change: main_item_callbacks_for_view.on_quantity_change,
                                        has_quantity: feats.iter().any(|f| f.name == FEATURE_QUANTITY),
                                        on_move: main_item_callbacks_for_view.on_move,
                                        on_date_save: main_item_callbacks_for_view.on_date_save,
                                        deadlines_config: get_deadlines_config(&feats),
                                        enable_item_dnd,
                                        item_dnd_state,
                                        on_item_drop: on_main_item_drop_for_view.clone(),
                                    }).into_any()
                                }
                            }}

                            {move || {
                                let main_list_id_for_subsections = main_list_id_for_sublist_view.clone();
                                let subs = sublists.get();
                                if subs.is_empty() {
                                    ().into_any()
                                } else {
                                    let sublists_for_render = subs.clone();
                                    view! {
                                        <div class="mt-6">
                                            {sublists_for_render.into_iter().map(|sl| {
                                                let drag_id = sl.id.clone();
                                                let drop_target = DropTarget::before(sl.id.clone());
                                                let drop_target_for_marker = drop_target.clone();
                                                let drop_target_for_surface = drop_target.clone();
                                                let drag_id_for_handle = drag_id.clone();
                                                let drag_id_for_shell = drag_id.clone();
                                                let drag_id_for_surface = drag_id.clone();
                                                let tags = all_tags.get();
                                                let links = item_tag_links.get();
                                                let lid = list_id();
                                                let lname = list_name.get();
                                                let sl_id = sl.id.clone();
                                                let sublist_id = sl.id.clone();
                                                let sibling_sublists = subs.clone();
                                                let mut mt: Vec<(String, String)> = vec![
                                                    (
                                                        lid,
                                                        move_tr!(
                                                            "lists-main-move-target",
                                                            { "name" => lname.clone() }
                                                        )
                                                        .get(),
                                                    )
                                                ];
                                                mt.extend(
                                                    sibling_sublists.iter()
                                                        .filter(|s| s.id != sl_id)
                                                        .map(|s| (s.id.clone(), s.name.clone()))
                                                );
                                                let feats = list_features.get();
                                                let deadlines_config = get_deadlines_config(&feats);
                                                let enable_item_dnd = active_tag_filter.get().is_none()
                                                    && !feats.iter().any(|feature| feature.name == FEATURE_DEADLINES);
                                                let scoped_callbacks = build_scoped_item_callbacks(
                                                    client.clone(),
                                                    main_list_id_for_subsections.clone(),
                                                    sublist_id.clone(),
                                                    items,
                                                    sublist_items,
                                                    set_error,
                                                    apply_item_drop_plan_for_view.clone(),
                                                );
                                                let sublist_scope_items = sublist_items
                                                    .get()
                                                    .get(&sublist_id)
                                                    .cloned()
                                                    .unwrap_or_default();
                                                view! {
                                                    <div class="flex flex-col gap-2">
                                                        <ReorderDropTarget
                                                            dnd_state=sublist_dnd_state
                                                            target=drop_target_for_marker
                                                            on_drop=on_sublist_drop.clone()
                                                        />
                                                        <DragShell dnd_state=sublist_dnd_state dragged_id=drag_id_for_shell>
                                                            <DragHandleButton
                                                                dnd_state=sublist_dnd_state
                                                                dragged_id=drag_id_for_handle
                                                                label=DragHandleLabel::ReorderGroup
                                                            />
                                                            <DragSurface
                                                                dnd_state=sublist_dnd_state
                                                                dragged_id=drag_id_for_surface
                                                                hover_target=drop_target_for_surface
                                                            >
                                                                <SublistSection
                                                                    sublist=sl.clone()
                                                                    items=sublist_scope_items
                                                                    enable_item_dnd=enable_item_dnd
                                                                    item_dnd_state=item_dnd_state
                                                                    on_item_drop=on_main_item_drop_for_view.clone()
                                                                    on_add=scoped_callbacks.on_add
                                                                    on_toggle=scoped_callbacks.on_toggle
                                                                    on_delete=scoped_callbacks.on_delete
                                                                    on_description_save=scoped_callbacks.on_description_save
                                                                    has_quantity=feats.iter().any(|f| f.name == FEATURE_QUANTITY)
                                                                    on_quantity_change=scoped_callbacks.on_quantity_change
                                                                    deadlines_config=deadlines_config
                                                                    all_tags=tags
                                                                    item_tag_links=links
                                                                    on_tag_toggle=on_tag_toggle
                                                                    move_targets=mt
                                                                    on_move=scoped_callbacks.on_move
                                                                    on_date_save=scoped_callbacks.on_date_save
                                                                />
                                                            </DragSurface>
                                                        </DragShell>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                            <ReorderDropTarget
                                                dnd_state=sublist_dnd_state
                                                target=DropTarget::end()
                                                on_drop=on_sublist_drop
                                            />
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

#[derive(Clone)]
struct ScopedItemCallbacks {
    on_add: Callback<CreateItemRequest>,
    on_toggle: Callback<String>,
    on_delete: Callback<String>,
    on_description_save: Callback<(String, String)>,
    on_quantity_change: Callback<(String, i32)>,
    on_move: Callback<(String, String)>,
    on_date_save: Callback<(String, String, String, Option<String>)>,
}

fn run_item_store_mutation<Mutate, Request, RequestFuture, OnError>(
    items: RwSignal<Vec<Item>>,
    sublist_items: RwSignal<HashMap<String, Vec<Item>>>,
    mutate: Mutate,
    request: Request,
    on_error: OnError,
) where
    Mutate: FnOnce(&mut Vec<Item>, &mut HashMap<String, Vec<Item>>) -> bool + 'static,
    Request: FnOnce() -> RequestFuture + 'static,
    RequestFuture: std::future::Future<Output = Result<(), crate::api::ApiError>> + 'static,
    OnError: FnOnce(crate::api::ApiError) + 'static,
{
    let previous_main = items.get_untracked();
    let previous_sub = sublist_items.get_untracked();

    let mut next_main = previous_main.clone();
    let mut next_sub = previous_sub.clone();
    if !mutate(&mut next_main, &mut next_sub) {
        return;
    }

    items.set(next_main);
    sublist_items.set(next_sub);

    leptos::task::spawn_local(async move {
        if let Err(error) = request().await {
            items.set(previous_main);
            sublist_items.set(previous_sub);
            on_error(error);
        }
    });
}

fn mutate_scope_items<F>(
    main_list_id: &str,
    scope_list_id: &str,
    items: &mut Vec<Item>,
    sublist_items: &mut HashMap<String, Vec<Item>>,
    mutate: F,
) -> bool
where
    F: FnOnce(&mut Vec<Item>) -> bool,
{
    if scope_list_id == main_list_id {
        mutate(items)
    } else {
        let scope_items = sublist_items.entry(scope_list_id.to_string()).or_default();
        mutate(scope_items)
    }
}

fn collect_item_ids_by_scope(
    main_list_id: &str,
    items: RwSignal<Vec<Item>>,
    sublist_items: RwSignal<HashMap<String, Vec<Item>>>,
) -> HashMap<String, Vec<String>> {
    let mut scopes = HashMap::from([(
        main_list_id.to_string(),
        items
            .get_untracked()
            .into_iter()
            .map(|item| item.id)
            .collect(),
    )]);
    for (scope_id, scope_items) in sublist_items.get_untracked() {
        scopes.insert(
            scope_id,
            scope_items.into_iter().map(|item| item.id).collect(),
        );
    }
    scopes
}

fn apply_item_drop_plan_locally(
    main_list_id: &str,
    items: &mut Vec<Item>,
    sublist_items: &mut HashMap<String, Vec<Item>>,
    plan: &ItemDropPlan,
) -> bool {
    match plan {
        ItemDropPlan::Reorder { list_id, item_ids } => {
            mutate_scope_items(main_list_id, list_id, items, sublist_items, |scope_items| {
                apply_reorder(scope_items, item_ids, |item| item.id.as_str())
            })
        }
        ItemDropPlan::Move { item_id, request } => {
            let source_list_id = request.source_list_id.as_str();
            let target_list_id = request.target_list_id.as_str();

            let removed_item = if source_list_id == main_list_id {
                let source_index = items.iter().position(|item| item.id == *item_id);
                source_index.map(|index| items.remove(index))
            } else {
                let Some(source_items) = sublist_items.get_mut(source_list_id) else {
                    return false;
                };
                let Some(source_index) = source_items.iter().position(|item| item.id == *item_id)
                else {
                    return false;
                };
                Some(source_items.remove(source_index))
            };

            let Some(mut moved_item) = removed_item else {
                return false;
            };
            moved_item.list_id = request.target_list_id.clone();

            if target_list_id == main_list_id {
                items.push(moved_item);
            } else {
                sublist_items
                    .entry(request.target_list_id.clone())
                    .or_default()
                    .push(moved_item);
            }

            let source_changed = mutate_scope_items(
                main_list_id,
                source_list_id,
                items,
                sublist_items,
                |scope_items| {
                    let current_ids: Vec<String> =
                        scope_items.iter().map(|item| item.id.clone()).collect();
                    current_ids == request.source_item_ids
                        || apply_reorder(scope_items, &request.source_item_ids, |item| {
                            item.id.as_str()
                        })
                },
            );
            let target_changed = mutate_scope_items(
                main_list_id,
                target_list_id,
                items,
                sublist_items,
                |scope_items| {
                    apply_reorder(scope_items, &request.target_item_ids, |item| {
                        item.id.as_str()
                    })
                },
            );

            source_changed || target_changed
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_scoped_item_callbacks(
    client: GlooClient,
    main_list_id: String,
    scope_list_id: String,
    items: RwSignal<Vec<Item>>,
    sublist_items: RwSignal<HashMap<String, Vec<Item>>>,
    set_error: WriteSignal<Option<String>>,
    apply_item_drop_plan: Callback<ItemDropPlan>,
) -> ScopedItemCallbacks {
    let scope_list_id_for_add = scope_list_id.clone();
    let client_for_add = client.clone();
    let main_list_id_for_add = main_list_id.clone();
    let on_add = Callback::new(move |req: CreateItemRequest| {
        let client = client_for_add.clone();
        let scope_list_id = scope_list_id_for_add.clone();
        let main_list_id = main_list_id_for_add.clone();
        leptos::task::spawn_local(async move {
            match api::create_item(&client, &scope_list_id, &req).await {
                Ok(item) => {
                    run_item_store_mutation(
                        items,
                        sublist_items,
                        move |main_items, sub_items| {
                            mutate_scope_items(
                                &main_list_id,
                                &scope_list_id,
                                main_items,
                                sub_items,
                                |scope_items| {
                                    scope_items.push(item.clone());
                                    true
                                },
                            )
                        },
                        || async { Ok(()) },
                        |_| {},
                    );
                }
                Err(error) => set_error.set(Some(error.to_string())),
            }
        });
    });

    let scope_list_id_for_toggle = scope_list_id.clone();
    let client_for_toggle = client.clone();
    let main_list_id_for_toggle = main_list_id.clone();
    let on_toggle = Callback::new(move |item_id: String| {
        let current_items = if scope_list_id_for_toggle == main_list_id_for_toggle {
            items.get_untracked()
        } else {
            sublist_items
                .get_untracked()
                .get(&scope_list_id_for_toggle)
                .cloned()
                .unwrap_or_default()
        };
        let (_next_items, new_completed) =
            crate::state::transforms::with_item_toggled(&current_items, &item_id);
        let Some(new_completed) = new_completed else {
            return;
        };

        let client = client_for_toggle.clone();
        let item_id_for_mutation = item_id.clone();
        let item_id_for_request = item_id.clone();
        let scope_list_id = scope_list_id_for_toggle.clone();
        let scope_list_id_for_request = scope_list_id.clone();
        let main_list_id = main_list_id_for_toggle.clone();
        run_item_store_mutation(
            items,
            sublist_items,
            move |main_items, sub_items| {
                mutate_scope_items(
                    &main_list_id,
                    &scope_list_id,
                    main_items,
                    sub_items,
                    |scope_items| {
                        let (next_items, changed_completed) =
                            crate::state::transforms::with_item_toggled(
                                scope_items,
                                &item_id_for_mutation,
                            );
                        if changed_completed.is_none() {
                            return false;
                        }
                        *scope_items = next_items;
                        true
                    },
                )
            },
            move || async move {
                let body = UpdateItemRequest {
                    completed: Some(new_completed),
                    ..Default::default()
                };
                api::update_item(
                    &client,
                    &scope_list_id_for_request,
                    &item_id_for_request,
                    &body,
                )
                .await
                .map(|_| ())
            },
            move |error| set_error.set(Some(error.to_string())),
        );
    });

    let scope_list_id_for_delete = scope_list_id.clone();
    let client_for_delete = client.clone();
    let main_list_id_for_delete = main_list_id.clone();
    let on_delete = Callback::new(move |item_id: String| {
        let client = client_for_delete.clone();
        let item_id_for_mutation = item_id.clone();
        let item_id_for_request = item_id.clone();
        let scope_list_id = scope_list_id_for_delete.clone();
        let scope_list_id_for_request = scope_list_id.clone();
        let main_list_id = main_list_id_for_delete.clone();
        run_item_store_mutation(
            items,
            sublist_items,
            move |main_items, sub_items| {
                mutate_scope_items(
                    &main_list_id,
                    &scope_list_id,
                    main_items,
                    sub_items,
                    |scope_items| {
                        let next_items = crate::state::transforms::without_item(
                            scope_items,
                            &item_id_for_mutation,
                        );
                        if next_items.len() == scope_items.len() {
                            return false;
                        }
                        *scope_items = next_items;
                        true
                    },
                )
            },
            move || async move {
                api::delete_item(&client, &scope_list_id_for_request, &item_id_for_request).await
            },
            move |error| set_error.set(Some(error.to_string())),
        );
    });

    let scope_list_id_for_description = scope_list_id.clone();
    let client_for_description = client.clone();
    let main_list_id_for_description = main_list_id.clone();
    let on_description_save = Callback::new(move |(item_id, new_desc): (String, String)| {
        let client = client_for_description.clone();
        let next_description = if new_desc.is_empty() {
            None
        } else {
            Some(new_desc.clone())
        };
        let item_id_for_mutation = item_id.clone();
        let item_id_for_request = item_id.clone();
        let scope_list_id = scope_list_id_for_description.clone();
        let scope_list_id_for_request = scope_list_id.clone();
        let main_list_id = main_list_id_for_description.clone();
        run_item_store_mutation(
            items,
            sublist_items,
            move |main_items, sub_items| {
                mutate_scope_items(
                    &main_list_id,
                    &scope_list_id,
                    main_items,
                    sub_items,
                    |scope_items| {
                        let Some(item) = scope_items
                            .iter_mut()
                            .find(|item| item.id == item_id_for_mutation)
                        else {
                            return false;
                        };
                        item.description = next_description.clone();
                        true
                    },
                )
            },
            move || async move {
                let req = UpdateItemRequest {
                    description: Some(if new_desc.is_empty() {
                        None
                    } else {
                        Some(new_desc)
                    }),
                    ..Default::default()
                };
                api::update_item(
                    &client,
                    &scope_list_id_for_request,
                    &item_id_for_request,
                    &req,
                )
                .await
                .map(|_| ())
            },
            move |error| set_error.set(Some(error.to_string())),
        );
    });

    let scope_list_id_for_quantity = scope_list_id.clone();
    let client_for_quantity = client.clone();
    let main_list_id_for_quantity = main_list_id.clone();
    let on_quantity_change = Callback::new(move |(item_id, new_actual): (String, i32)| {
        let client = client_for_quantity.clone();
        let item_id_for_request = item_id.clone();
        let scope_list_id = scope_list_id_for_quantity.clone();
        let scope_list_id_for_request = scope_list_id.clone();
        let main_list_id = main_list_id_for_quantity.clone();
        run_item_store_mutation(
            items,
            sublist_items,
            move |main_items, sub_items| {
                mutate_scope_items(
                    &main_list_id,
                    &scope_list_id,
                    main_items,
                    sub_items,
                    |scope_items| {
                        let Some(item) = scope_items.iter_mut().find(|item| item.id == item_id)
                        else {
                            return false;
                        };
                        item.actual_quantity = Some(new_actual);
                        if let Some(target_quantity) = item.quantity {
                            item.completed = new_actual >= target_quantity;
                        }
                        true
                    },
                )
            },
            move || async move {
                let req = UpdateItemRequest {
                    actual_quantity: Some(new_actual),
                    ..Default::default()
                };
                api::update_item(
                    &client,
                    &scope_list_id_for_request,
                    &item_id_for_request,
                    &req,
                )
                .await
                .map(|_| ())
            },
            move |error| set_error.set(Some(error.to_string())),
        );
    });

    let scope_list_id_for_move = scope_list_id.clone();
    let main_list_id_for_move = main_list_id.clone();
    let on_move = Callback::new(move |(item_id, target_list_id): (String, String)| {
        let dragged_item = DraggedItem {
            item_id,
            source_list_id: scope_list_id_for_move.clone(),
        };
        let item_ids_by_scope =
            collect_item_ids_by_scope(&main_list_id_for_move, items, sublist_items);
        let Some(plan) = build_item_drop_plan(
            &item_ids_by_scope,
            &dragged_item,
            &ItemDropTarget::end(target_list_id),
        ) else {
            return;
        };
        apply_item_drop_plan.run(plan);
    });

    let scope_list_id_for_date = scope_list_id;
    let client_for_date = client;
    let main_list_id_for_date = main_list_id;
    let on_date_save = Callback::new(
        move |(item_id, date_type, date_val, time_val): (
            String,
            String,
            String,
            Option<String>,
        )| {
            let Some(field) = ItemDateField::parse(&date_type) else {
                return;
            };
            let Some(request) = build_date_update_request(&date_type, &date_val, time_val.clone())
            else {
                return;
            };

            let client = client_for_date.clone();
            let item_id_for_mutation = item_id.clone();
            let item_id_for_request = item_id.clone();
            let time_for_mutation = time_val.clone();
            let scope_list_id = scope_list_id_for_date.clone();
            let scope_list_id_for_request = scope_list_id.clone();
            let main_list_id = main_list_id_for_date.clone();
            run_item_store_mutation(
                items,
                sublist_items,
                move |main_items, sub_items| {
                    mutate_scope_items(
                        &main_list_id,
                        &scope_list_id,
                        main_items,
                        sub_items,
                        |scope_items| {
                            apply_date_change_to_items(
                                scope_items,
                                &item_id_for_mutation,
                                field,
                                &date_val,
                                time_for_mutation.as_deref(),
                            )
                        },
                    )
                },
                move || async move {
                    api::update_item(
                        &client,
                        &scope_list_id_for_request,
                        &item_id_for_request,
                        &request,
                    )
                    .await
                    .map(|_| ())
                },
                move |error| set_error.set(Some(error.to_string())),
            );
        },
    );

    ScopedItemCallbacks {
        on_add,
        on_toggle,
        on_delete,
        on_description_save,
        on_quantity_change,
        on_move,
        on_date_save,
    }
}
