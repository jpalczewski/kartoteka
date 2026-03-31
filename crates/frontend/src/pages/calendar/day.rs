use std::collections::{BTreeMap, HashSet};

use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::api;
use crate::api::client::GlooClient;
use crate::app::{ToastContext, ToastKind};
use crate::components::common::date_utils::{
    add_days, day_of_week, format_polish_date, polish_day_of_week_full,
};
use crate::components::common::loading::LoadingSpinner;
use crate::components::filters::filter_chips::FilterChips;
use crate::components::items::date_item_row::DateItemRow;
use crate::components::tags::tag_tree::build_tag_filter_options;
use kartoteka_shared::*;

#[component]
pub fn CalendarDayPage() -> impl IntoView {
    let params = use_params_map();
    let date = move || params.read().get("date").unwrap_or_default();

    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let client = use_context::<GlooClient>().expect("GlooClient not provided");
    let items = RwSignal::new(Vec::<DateItem>::new());
    let all_tags = RwSignal::new(Vec::<Tag>::new());
    let item_tag_links = RwSignal::new(Vec::<ItemTagLink>::new());
    let (loading, set_loading) = signal(true);
    let hidden_lists = RwSignal::new(HashSet::<String>::new());
    let hidden_tags = RwSignal::new(HashSet::<String>::new());
    let show_completed = RwSignal::new(true);

    let _resource = {
        let client = client.clone();
        LocalResource::new(move || {
            let d = date();
            let client = client.clone();
            async move {
                set_loading.set(true);
                match api::fetch_items_by_date(&client, &d, false, "all").await {
                    Ok(fetched) => items.set(fetched),
                    Err(e) => toast.push(format!("Błąd: {e}"), ToastKind::Error),
                }
                if let Ok(tags) = api::fetch_tags(&client).await {
                    all_tags.set(tags);
                }
                if let Ok(links) = api::fetch_item_tag_links(&client).await {
                    item_tag_links.set(links);
                }
                set_loading.set(false);
            }
        })
    };

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            // Header with date + navigation
            {move || {
                let d = date();
                let prev = add_days(&d, -1);
                let next = add_days(&d, 1);
                let dow = day_of_week(&d);
                let dow_name = polish_day_of_week_full(dow);
                let formatted = format_polish_date(&d);

                view! {
                    <div class="flex items-center justify-between mb-4">
                        <A href=format!("/calendar/{prev}") attr:class="btn btn-sm btn-ghost">"‹"</A>
                        <div class="text-center">
                            <h1 class="text-2xl font-bold">{formatted}</h1>
                            <span class="text-base-content/50 capitalize">{dow_name}</span>
                        </div>
                        <A href=format!("/calendar/{next}") attr:class="btn btn-sm btn-ghost">"›"</A>
                    </div>
                    <div class="text-center mb-4">
                        <A href="/calendar" attr:class="btn btn-sm btn-outline btn-ghost">{move_tr!("calendar-back-to-calendar")}</A>
                    </div>
                }
            }}

            {move || {
                if loading.get() {
                    return view! { <LoadingSpinner/> }.into_any();
                }

                let all_items = items.get();
                let tags = all_tags.get();
                let links = item_tag_links.get();

                if all_items.is_empty() {
                    return view! {
                        <p class="text-center text-base-content/50 py-12">
                            {move_tr!("calendar-empty-day")}
                        </p>
                    }.into_any();
                }

                // Collect unique lists
                let mut unique_lists: Vec<(String, String)> = Vec::new();
                let mut seen = HashSet::new();
                for item in &all_items {
                    if seen.insert(item.list_id.clone()) {
                        unique_lists.push((item.list_id.clone(), item.list_name.clone()));
                    }
                }

                // Collect relevant tags
                let item_ids: HashSet<String> = all_items.iter().map(|i| i.id.clone()).collect();
                let relevant_tag_ids: HashSet<String> = links.iter()
                    .filter(|l| item_ids.contains(&l.item_id))
                    .map(|l| l.tag_id.clone())
                    .collect();
                let relevant_tag_ids: Vec<String> = relevant_tag_ids.into_iter().collect();
                let relevant_tags = build_tag_filter_options(&tags, &relevant_tag_ids);

                // Filter
                let hl = hidden_lists.get();
                let ht = hidden_tags.get();
                let sc = show_completed.get();

                let filtered: Vec<DateItem> = all_items.into_iter()
                    .filter(|item| {
                        if hl.contains(&item.list_id) { return false; }
                        if !sc && item.completed { return false; }
                        if !ht.is_empty() {
                            let item_tags: HashSet<String> = links.iter()
                                .filter(|l| l.item_id == item.id)
                                .map(|l| l.tag_id.clone())
                                .collect();
                            if ht.iter().any(|t| item_tags.contains(t)) {
                                return false;
                            }
                        }
                        true
                    })
                    .collect();

                // Group by list
                let mut groups: BTreeMap<(String, String), Vec<DateItem>> = BTreeMap::new();
                for item in filtered {
                    let key = (item.list_id.clone(), item.list_name.clone());
                    groups.entry(key).or_default().push(item);
                }

                let client_render = use_context::<GlooClient>().expect("GlooClient not provided");

                view! {
                    <div>
                        <FilterChips
                            unique_lists=unique_lists
                            relevant_tags=relevant_tags
                            hidden_lists=hidden_lists
                            hidden_tags=hidden_tags
                            show_completed=show_completed
                        />

                        {groups.into_iter().map(|((list_id, list_name), group_items)| {
                            let tags = tags.clone();
                            let links = links.clone();
                            let client = client_render.clone();
                            view! {
                                <div class="mb-4">
                                    <h4 class="text-sm font-semibold uppercase tracking-wide mb-1 text-base-content/70">
                                        <A href=format!("/lists/{list_id}") attr:class="link link-hover">
                                            {list_name}
                                        </A>
                                    </h4>
                                    {group_items.into_iter().map(|date_item| {
                                        let item_id = date_item.id.clone();
                                        let item_list_id = date_item.list_id.clone();
                                        let date_type = date_item.date_type.clone();
                                        let item: Item = date_item.into();

                                        let item_tag_ids: Vec<String> = links.iter()
                                            .filter(|l| l.item_id == item_id)
                                            .map(|l| l.tag_id.clone())
                                            .collect();

                                        let toggle_list_id = item_list_id.clone();
                                        let toggle_item_id = item_id.clone();
                                        let client_toggle = client.clone();
                                        let on_toggle = Callback::new(move |_id: String| {
                                            let lid = toggle_list_id.clone();
                                            let iid = toggle_item_id.clone();
                                            let client_t = client_toggle.clone();
                                            items.update(|items| {
                                                if let Some(item) = items.iter_mut().find(|i| i.id == iid) {
                                                    item.completed = !item.completed;
                                                }
                                            });
                                            leptos::task::spawn_local(async move {
                                                let current = items.get_untracked()
                                                    .iter()
                                                    .find(|i| i.id == iid)
                                                    .map(|i| i.completed)
                                                    .unwrap_or(false);
                                                let req = UpdateItemRequest {
                                                    title: None,
                                                    description: None,
                                                    completed: Some(current),
                                                    position: None,
                                                    quantity: None,
                                                    actual_quantity: None,
                                                    unit: None,
                                                    start_date: None,
                                                    start_time: None,
                                                    deadline: None,
                                                    deadline_time: None,
                                                    hard_deadline: None,
                                                };
                                                let _ = api::update_item(&client_t, &lid, &iid, &req).await;
                                            });
                                        });

                                        let delete_list_id = item_list_id.clone();
                                        let delete_item_id = item_id.clone();
                                        let client_delete = client.clone();
                                        let on_delete = Callback::new(move |_id: String| {
                                            let lid = delete_list_id.clone();
                                            let iid = delete_item_id.clone();
                                            let client_d = client_delete.clone();
                                            items.update(|items| {
                                                items.retain(|i| i.id != iid);
                                            });
                                            leptos::task::spawn_local(async move {
                                                let _ = api::delete_item(&client_d, &lid, &iid).await;
                                            });
                                        });

                                        // Date save
                                        let ds_lid = item_list_id.clone();
                                        let ds_iid = item_id.clone();
                                        let client_date = client.clone();
                                        let on_date_save = Callback::new(move |(_iid, dt, date, time): (String, String, String, Option<String>)| {
                                            let lid = ds_lid.clone();
                                            let iid = ds_iid.clone();
                                            let client_ds = client_date.clone();
                                            let date_opt = if date.is_empty() { Some(None) } else { Some(Some(date)) };
                                            let time_opt = if date_opt == Some(None) { Some(None) } else { time.map(Some) };
                                            leptos::task::spawn_local(async move {
                                                let mut req = UpdateItemRequest {
                                                    title: None, description: None, completed: None, position: None,
                                                    quantity: None, actual_quantity: None, unit: None,
                                                    start_date: None, start_time: None,
                                                    deadline: None, deadline_time: None, hard_deadline: None,
                                                };
                                                match dt.as_str() {
                                                    "start" => { req.start_date = date_opt; req.start_time = time_opt; }
                                                    "deadline" => { req.deadline = date_opt; req.deadline_time = time_opt; }
                                                    "hard_deadline" => { req.hard_deadline = date_opt; }
                                                    _ => {}
                                                }
                                                let _ = api::update_item(&client_ds, &lid, &iid, &req).await;
                                            });
                                        });

                                        {if let Some(dt) = date_type {
                                            view! {
                                                <DateItemRow
                                                    item=item
                                                    on_toggle=on_toggle
                                                    on_delete=on_delete
                                                    all_tags=tags.clone()
                                                    item_tag_ids=item_tag_ids
                                                    date_type=dt
                                                    on_date_save=on_date_save
                                                />
                                            }.into_any()
                                        } else {
                                            view! {
                                                <DateItemRow
                                                    item=item
                                                    on_toggle=on_toggle
                                                    on_delete=on_delete
                                                    all_tags=tags.clone()
                                                    item_tag_ids=item_tag_ids
                                                    on_date_save=on_date_save
                                                />
                                            }.into_any()
                                        }}
                                    }).collect_view()}
                                </div>
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </div>
    }
    .into_any()
}
