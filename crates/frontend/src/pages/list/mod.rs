mod date_view;
mod normal_view;

use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::api;
use crate::app::{ToastContext, ToastKind};
use crate::components::common::breadcrumbs::Breadcrumbs;
use crate::components::common::loading::LoadingSpinner;
use crate::components::common::editable_description::EditableDescription;
use crate::components::items::add_item_input::AddItemInput;
use crate::components::items::item_actions::create_item_actions;
use crate::components::lists::add_group_input::AddGroupInput;
use crate::components::lists::list_header::ListHeader;
use crate::components::lists::list_tag_bar::ListTagBar;
use crate::components::lists::sublist_section::SublistSection;
use crate::components::tags::tag_filter_bar::TagFilterBar;
use kartoteka_shared::{
    FEATURE_DEADLINES, FEATURE_QUANTITY, Item, ItemTagLink, List, ListFeature, ListTagLink, Tag,
    UpdateListRequest,
};

use date_view::render_date_view;
use normal_view::{NormalViewProps, render_normal_view};

#[component]
pub fn ListPage() -> impl IntoView {
    let params = use_params_map();
    let list_id = move || params.read().get("id").unwrap_or_default();

    let items = RwSignal::new(Vec::<Item>::new());
    let all_tags = RwSignal::new(Vec::<Tag>::new());
    let item_tag_links = RwSignal::new(Vec::<ItemTagLink>::new());
    let list_tag_links = RwSignal::new(Vec::<ListTagLink>::new());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(Option::<String>::None);
    let breadcrumbs = RwSignal::new(Vec::<(String, String)>::new());

    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let navigate = use_navigate();
    let list_name = RwSignal::new(String::new());
    let list_description = RwSignal::new(Option::<String>::None);
    let list_features = RwSignal::new(Vec::<ListFeature>::new());
    let sublists = RwSignal::new(Vec::<List>::new());

    let lid = list_id();
    leptos::task::spawn_local(async move {
        if let Ok(list) = api::fetch_list(&lid).await {
            // Build breadcrumbs if the list belongs to a container
            if let Some(cid) = list.container_id.clone() {
                if let Ok(all_containers) = api::fetch_containers().await {
                    let mut chain = Vec::new();
                    let mut current_id = Some(cid);
                    let mut depth = 0;
                    while let Some(ref id) = current_id.clone() {
                        if depth > 10 {
                            break;
                        }
                        if let Some(c) = all_containers.iter().find(|c| &c.id == id) {
                            chain.push((c.name.clone(), format!("/containers/{}", c.id)));
                            current_id = c.parent_container_id.clone();
                        } else {
                            break;
                        }
                        depth += 1;
                    }
                    chain.reverse();
                    breadcrumbs.set(chain);
                }
            }
            list_name.set(list.name);
            list_description.set(list.description);
            list_features.set(list.features);
        }
        match api::fetch_items(&lid).await {
            Ok(fetched) => items.set(fetched),
            Err(e) => set_error.set(Some(e)),
        }
        if let Ok(fetched_tags) = api::fetch_tags().await {
            all_tags.set(fetched_tags);
        }
        if let Ok(links) = api::fetch_item_tag_links().await {
            item_tag_links.set(links);
        }
        if let Ok(fetched_sublists) = api::fetch_sublists(&lid).await {
            sublists.set(fetched_sublists);
        }
        if let Ok(links) = api::fetch_list_tag_links().await {
            let filtered: Vec<ListTagLink> =
                links.into_iter().filter(|l| l.list_id == lid).collect();
            list_tag_links.set(filtered);
        }
        set_loading.set(false);
    });

    let actions = create_item_actions(items, list_id(), Some(set_error));
    let on_add = actions.on_add;
    let on_toggle = actions.on_toggle;
    let on_delete = actions.on_delete;
    let on_description_save = actions.on_description_save;
    let on_quantity_change = actions.on_quantity_change;
    let on_date_save = actions.on_date_save;

    let on_tag_toggle = make_item_tag_toggle(item_tag_links);
    let lid_for_tag = list_id();
    let on_list_tag_toggle = make_list_tag_toggle(list_tag_links, lid_for_tag);
    let on_move_main = make_move_callback(items);

    let parent_lid = list_id();
    let on_item_moved_out = Callback::new(move |(moved_item, target_list_id): (Item, String)| {
        if target_list_id == parent_lid {
            items.update(|list| list.push(moved_item));
        }
    });

    let on_delete_list = {
        let navigate = navigate.clone();
        Callback::new(move |_: ()| {
            let lid = list_id();
            let nav = navigate.clone();
            leptos::task::spawn_local(async move {
                match api::delete_list(&lid).await {
                    Ok(()) => {
                        toast.push("Lista usunięta".into(), ToastKind::Success);
                        nav("/", Default::default());
                    }
                    Err(e) => toast.push(e, ToastKind::Error),
                }
            });
        })
    };

    let on_archive = {
        let navigate = use_navigate();
        Callback::new(move |_: ()| {
            let lid = list_id();
            let nav = navigate.clone();
            leptos::task::spawn_local(async move {
                match api::archive_list(&lid).await {
                    Ok(_) => {
                        toast.push("Lista zarchiwizowana".into(), ToastKind::Success);
                        nav("/", Default::default());
                    }
                    Err(e) => toast.push(e, ToastKind::Error),
                }
            });
        })
    };

    let on_reset = Callback::new(move |_: ()| {
        let lid = list_id();
        leptos::task::spawn_local(async move {
            match api::reset_list(&lid).await {
                Ok(()) => {
                    items.update(|list| {
                        for item in list.iter_mut() {
                            item.completed = false;
                            item.actual_quantity = Some(0);
                        }
                    });
                    toast.push("Lista zresetowana".into(), ToastKind::Success);
                }
                Err(e) => toast.push(e, ToastKind::Error),
            }
        });
    });

    let on_create_group = Callback::new(move |name: String| {
        let lid = list_id();
        leptos::task::spawn_local(async move {
            if let Ok(sl) = api::create_sublist(&lid, &name).await {
                sublists.update(|list| list.push(sl));
            }
        });
    });

    let (active_tag_filter, set_active_tag_filter) = signal(Option::<String>::None);

    let sorted_items = move || {
        let mut list = items.get();
        list.sort_by(|a, b| {
            a.completed
                .cmp(&b.completed)
                .then(a.position.cmp(&b.position))
        });
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

            {move || view! {
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
                                let _ = api::add_feature(&lid, &fname, config).await;
                            } else {
                                let _ = api::remove_feature(&lid, &fname).await;
                            }
                        });
                    })
                    on_deadlines_config_change=Callback::new(move |new_config: serde_json::Value| {
                        let lid = list_id();
                        // Optimistic update
                        list_features.update(|feats| {
                            if let Some(f) = feats.iter_mut().find(|f| f.name == FEATURE_DEADLINES) {
                                f.config = new_config.clone();
                            }
                        });
                        leptos::task::spawn_local(async move {
                            let _ = api::add_feature(&lid, FEATURE_DEADLINES, new_config).await;
                        });
                    })
                    on_rename=Callback::new(move |new_name: String| {
                        list_name.set(new_name.clone());
                        let lid = list_id();
                        leptos::task::spawn_local(async move {
                            let req = UpdateListRequest {
                                name: Some(new_name),
                                description: None,
                                list_type: None,
                                archived: None,
                            };
                            let _ = api::update_list(&lid, &req).await;
                        });
                    })
                />
            }}

            {move || view! {
                <EditableDescription
                    value=list_description.get()
                    on_save=Callback::new(move |new_desc: Option<String>| {
                        list_description.set(new_desc.clone());
                        let lid = list_id();
                        leptos::task::spawn_local(async move {
                            let req = UpdateListRequest {
                                name: None,
                                description: new_desc,
                                list_type: None,
                                archived: None,
                            };
                            let _ = api::update_list(&lid, &req).await;
                        });
                    })
                />
            }}

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
                let tags = all_tags.read();
                view! { <TagFilterBar tags=tags.clone() active_tag_id=active_tag_filter on_select=set_active_tag_filter /> }
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
                                                }
                                            }).collect::<Vec<_>>()}
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

fn make_item_tag_toggle(item_tag_links: RwSignal<Vec<ItemTagLink>>) -> Callback<(String, String)> {
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
            leptos::task::spawn_local(async move {
                let _ = api::remove_tag_from_item(&iid, &tid).await;
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
            leptos::task::spawn_local(async move {
                let _ = api::assign_tag_to_item(&iid, &tid).await;
            });
        }
    })
}

fn make_list_tag_toggle(
    list_tag_links: RwSignal<Vec<ListTagLink>>,
    list_id: String,
) -> Callback<String> {
    Callback::new(move |tag_id: String| {
        let has_tag = list_tag_links.read().iter().any(|l| l.tag_id == tag_id);
        if has_tag {
            list_tag_links.update(|links| links.retain(|l| l.tag_id != tag_id));
            let lid = list_id.clone();
            let tid = tag_id.clone();
            leptos::task::spawn_local(async move {
                let _ = api::remove_tag_from_list(&lid, &tid).await;
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
            leptos::task::spawn_local(async move {
                let _ = api::assign_tag_to_list(&lid, &tid).await;
            });
        }
    })
}

fn make_move_callback(items: RwSignal<Vec<Item>>) -> Callback<(String, String)> {
    Callback::new(move |(item_id, target_list_id): (String, String)| {
        items.update(|list| list.retain(|i| i.id != item_id));
        leptos::task::spawn_local(async move {
            let _ = api::move_item(&item_id, &target_list_id).await;
        });
    })
}
