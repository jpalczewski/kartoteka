use std::collections::{BTreeMap, HashSet};

use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::components::A;

use crate::api;
use crate::api::client::GlooClient;
use crate::app::{ToastContext, ToastKind};
use crate::components::common::date_utils::{
    format_polish_date, get_today_string, is_overdue_for_date_type,
};
use crate::components::common::loading::LoadingSpinner;
use crate::components::filters::filter_chips::FilterChips;
use crate::components::items::date_item_row::DateItemRow;
use crate::components::tags::tag_tree::build_tag_filter_options;
use crate::state::item_mutations::{
    ItemDateField, apply_date_change_to_date_items, build_date_update_request,
    run_optimistic_mutation,
};
use kartoteka_shared::*;

#[component]
pub fn TodayPage() -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let client = use_context::<GlooClient>().expect("GlooClient not provided");
    let today = get_today_string();
    let items = RwSignal::new(Vec::<DateItem>::new());
    let all_tags = RwSignal::new(Vec::<Tag>::new());
    let item_tag_links = RwSignal::new(Vec::<ItemTagLink>::new());
    let (loading, set_loading) = signal(true);
    let hidden_lists = RwSignal::new(HashSet::<String>::new());
    let hidden_tags = RwSignal::new(HashSet::<String>::new());
    let show_completed = RwSignal::new(true);

    let today_for_fetch = today.clone();
    let _resource = {
        let client = client.clone();
        LocalResource::new(move || {
            let date = today_for_fetch.clone();
            let client = client.clone();
            async move {
                match api::fetch_items_by_date(&client, &date, true, "all").await {
                    Ok(fetched) => items.set(fetched),
                    Err(e) => toast.push(format!("Błąd ładowania zadań: {e}"), ToastKind::Error),
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

    let today_display = today.clone();
    let today_for_overdue = today.clone();

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <div class="flex items-center justify-between mb-4">
                <h1 class="text-2xl font-bold">{move_tr!("today-title")}</h1>
                <span class="text-base-content/50">{format_polish_date(&today_display)}</span>
            </div>

            {move || {
                if loading.get() {
                    return view! { <LoadingSpinner/> }.into_any();
                }

                let all_items = items.get();
                let tags = all_tags.get();
                let links = item_tag_links.get();
                let today_str = today_for_overdue.clone();

                if all_items.is_empty() {
                    return view! {
                        <p class="text-center text-base-content/50 py-12">
                            {move_tr!("today-empty")}
                        </p>
                    }.into_any();
                }

                // Collect unique lists from items
                let mut unique_lists: Vec<(String, String)> = Vec::new();
                let mut seen_lists = HashSet::new();
                for item in &all_items {
                    if seen_lists.insert(item.list_id.clone()) {
                        unique_lists.push((item.list_id.clone(), item.list_name.clone()));
                    }
                }

                // Collect unique tags that appear on these items
                let item_ids: HashSet<String> = all_items.iter().map(|i| i.id.clone()).collect();
                let relevant_tag_ids: HashSet<String> = links.iter()
                    .filter(|l| item_ids.contains(&l.item_id))
                    .map(|l| l.tag_id.clone())
                    .collect();
                let relevant_tag_ids: Vec<String> = relevant_tag_ids.into_iter().collect();
                let relevant_tags = build_tag_filter_options(&tags, &relevant_tag_ids);

                // Filter items
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

                // Split into overdue and today — check the relevant date field per date_type
                let mut overdue_groups: BTreeMap<(String, String), Vec<DateItem>> = BTreeMap::new();
                let mut today_groups: BTreeMap<(String, String), Vec<DateItem>> = BTreeMap::new();

                for item in filtered {
                    let relevant_date = match item.date_type.as_deref() {
                        Some("start") => item.start_date.as_deref(),
                        Some("hard_deadline") => item.hard_deadline.as_deref(),
                        _ => item.deadline.as_deref(),
                    };
                    let is_overdue = is_overdue_for_date_type(relevant_date, item.completed, &today_str);
                    let key = (item.list_id.clone(), item.list_name.clone());
                    if is_overdue {
                        overdue_groups.entry(key).or_default().push(item);
                    } else {
                        today_groups.entry(key).or_default().push(item);
                    }
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

                        // Overdue section
                        {
                            let has_overdue = !overdue_groups.is_empty();
                            let has_today = !today_groups.is_empty();

                            let overdue_view = if has_overdue {
                                view! {
                                    <div class="mb-6">
                                        <h3 class="text-xs text-error uppercase tracking-wider font-semibold mb-2">
                                            {move_tr!("today-overdue")}
                                        </h3>
                                        {render_groups(overdue_groups, tags.clone(), links.clone(), items, client_render.clone(), toast.clone())}
                                    </div>
                                }.into_any()
                            } else {
                                ().into_any()
                            };

                            let today_view = if has_today {
                                view! {
                                    <div class="mb-6">
                                        {if has_overdue {
                                            view! {
                                                <h3 class="text-xs text-base-content/50 uppercase tracking-wider font-semibold mb-2">
                                                    {move_tr!("today-title")}
                                                </h3>
                                            }.into_any()
                                        } else {
                                            ().into_any()
                                        }}
                                        {render_groups(today_groups, tags.clone(), links.clone(), items, client_render.clone(), toast.clone())}
                                    </div>
                                }.into_any()
                            } else {
                                ().into_any()
                            };

                            view! {
                                {overdue_view}
                                {today_view}
                            }
                        }
                    </div>
                }.into_any()
            }}
        </div>
    }
    .into_any()
}

fn render_groups(
    groups: BTreeMap<(String, String), Vec<DateItem>>,
    all_tags: Vec<Tag>,
    all_links: Vec<ItemTagLink>,
    items_signal: RwSignal<Vec<DateItem>>,
    client: GlooClient,
    toast: ToastContext,
) -> impl IntoView {
    groups
        .into_iter()
        .map(|((list_id, list_name), group_items)| {
            let tags = all_tags.clone();
            let links = all_links.clone();
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
                        let toast_toggle = toast.clone();
                        let on_toggle = Callback::new(move |_id: String| {
                            let lid = toggle_list_id.clone();
                            let iid = toggle_item_id.clone();
                            let client_t = client_toggle.clone();
                            let new_completed = items_signal
                                .get_untracked()
                                .iter()
                                .find(|i| i.id == iid)
                                .map(|i| !i.completed);
                            let Some(new_completed) = new_completed else { return };
                            let iid_for_mutation = iid.clone();
                            let iid_for_request = iid.clone();
                            run_optimistic_mutation(
                                items_signal,
                                move |items| {
                                    let mut changed = false;
                                    for item in items
                                        .iter_mut()
                                        .filter(|item| item.id == iid_for_mutation)
                                    {
                                        item.completed = new_completed;
                                        changed = true;
                                    }
                                    changed
                                },
                                move || async move {
                                    let req = UpdateItemRequest {
                                        completed: Some(new_completed),
                                        ..Default::default()
                                    };
                                    api::update_item(&client_t, &lid, &iid_for_request, &req)
                                        .await
                                        .map(|_| ())
                                },
                                move |e| toast_toggle.push(format!("Błąd: {e}"), ToastKind::Error),
                            );
                        });

                        let delete_list_id = item_list_id.clone();
                        let delete_item_id = item_id.clone();
                        let client_delete = client.clone();
                        let toast_delete = toast.clone();
                        let on_delete = Callback::new(move |_id: String| {
                            let lid = delete_list_id.clone();
                            let iid = delete_item_id.clone();
                            let client_d = client_delete.clone();
                            let iid_for_mutation = iid.clone();
                            let iid_for_request = iid.clone();
                            run_optimistic_mutation(
                                items_signal,
                                move |items| {
                                    let before_len = items.len();
                                    items.retain(|item| item.id != iid_for_mutation);
                                    items.len() != before_len
                                },
                                move || async move { api::delete_item(&client_d, &lid, &iid_for_request).await },
                                move |e| toast_delete.push(format!("Błąd: {e}"), ToastKind::Error),
                            );
                        });

                        // Date save callback
                        let date_save_list_id = item_list_id.clone();
                        let date_save_item_id = item_id.clone();
                        let client_date = client.clone();
                        let toast_date = toast.clone();
                        let on_date_save = Callback::new(move |(iid, dt, date, time): (String, String, String, Option<String>)| {
                            let Some(field) = ItemDateField::parse(&dt) else { return; };
                            let Some(req) = build_date_update_request(&dt, &date, time.clone()) else { return; };
                            let lid = date_save_list_id.clone();
                            let client_ds = client_date.clone();
                            let iid2 = date_save_item_id.clone();
                            let time_for_mutation = time.clone();
                            run_optimistic_mutation(
                                items_signal,
                                move |items| {
                                    apply_date_change_to_date_items(
                                        items,
                                        &iid,
                                        field,
                                        &date,
                                        time_for_mutation.as_deref(),
                                    )
                                },
                                move || async move {
                                    api::update_item(&client_ds, &lid, &iid2, &req)
                                        .await
                                        .map(|_| ())
                                },
                                move |e| toast_date.push(format!("Błąd: {e}"), ToastKind::Error),
                            );
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
        })
        .collect_view()
}
