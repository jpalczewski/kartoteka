use std::collections::{BTreeMap, HashSet};
use std::future::Future;

use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};

use crate::api;
use crate::api::ApiError;
use crate::api::client::GlooClient;
use crate::app::{ToastContext, ToastKind};
use crate::components::common::date_utils::{
    add_days, day_of_week, format_polish_date, parse_date, polish_day_of_week_full,
};
use crate::components::common::loading::LoadingSpinner;
use crate::components::items::date_item_row::DateItemRow;
use crate::state::calendar_route::{CalendarRouteState, calendar_href};
use crate::state::item_mutations::{
    ItemDateField, apply_date_change_to_date_items, apply_date_change_to_day_items,
    build_date_update_request, run_optimistic_mutation,
};
use kartoteka_shared::*;

fn run_dual_optimistic_mutation<T, U, MutateFirst, MutateSecond, Request, RequestFuture, OnError>(
    first: RwSignal<T>,
    second: RwSignal<U>,
    mutate_first: MutateFirst,
    mutate_second: MutateSecond,
    request: Request,
    on_error: OnError,
) where
    T: Clone + Send + Sync + 'static,
    U: Clone + Send + Sync + 'static,
    MutateFirst: FnOnce(&mut T) -> bool + 'static,
    MutateSecond: FnOnce(&mut U) + 'static,
    Request: FnOnce() -> RequestFuture + 'static,
    RequestFuture: Future<Output = Result<(), ApiError>> + 'static,
    OnError: FnOnce(ApiError) + 'static,
{
    let previous_first = first.get_untracked();
    let previous_second = second.get_untracked();

    let mut next_first = previous_first.clone();
    if !mutate_first(&mut next_first) {
        return;
    }

    let mut next_second = previous_second.clone();
    mutate_second(&mut next_second);

    first.set(next_first);
    second.set(next_second);

    leptos::task::spawn_local(async move {
        if let Err(error) = request().await {
            first.set(previous_first);
            second.set(previous_second);
            on_error(error);
        }
    });
}

#[component]
pub fn CalendarLegacyDayRedirect() -> impl IntoView {
    let params = use_params_map();
    let query = use_query_map();
    let navigate = use_navigate();
    let today = crate::components::common::date_utils::get_today_string();

    Effect::new(move |_| {
        let legacy_date = params.read().get("date").unwrap_or_default();
        let selected_date = parse_date(&legacy_date)
            .map(|date| date.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| today.clone());
        let query = query.read();
        let state = CalendarRouteState {
            selected_date,
            view_mode: crate::components::calendar::ViewMode::parse(
                query.get_str("view").unwrap_or("month"),
            ),
            hidden_lists: query
                .get_str("hidden_lists")
                .unwrap_or_default()
                .split(',')
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            hidden_tags: query
                .get_str("hidden_tags")
                .unwrap_or_default()
                .split(',')
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            show_completed: query.get_str("show_completed").unwrap_or("1") != "0",
        };

        navigate(&calendar_href(&state), Default::default());
    });

    view! { <LoadingSpinner/> }
}

#[component]
pub fn CalendarDayPanel(
    selected_date: RwSignal<String>,
    items: RwSignal<Vec<DateItem>>,
    loading: ReadSignal<bool>,
    all_tags: RwSignal<Vec<Tag>>,
    item_tag_links: RwSignal<Vec<ItemTagLink>>,
    hidden_lists: RwSignal<HashSet<String>>,
    hidden_tags: RwSignal<HashSet<String>>,
    show_completed: RwSignal<bool>,
    on_select_date: Callback<String>,
    week_items: RwSignal<Vec<DayItems>>,
    sync_week: Signal<bool>,
) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let client = use_context::<GlooClient>().expect("GlooClient not provided");

    view! {
        <div class="mt-6 rounded-xl border border-base-300 bg-base-100 p-4">
            {move || {
                let date = selected_date.get();
                let prev = add_days(&date, -1);
                let next = add_days(&date, 1);
                let dow = day_of_week(&date);
                let dow_name = polish_day_of_week_full(dow);
                let formatted = format_polish_date(&date);

                view! {
                    <div class="mb-4 flex items-center justify-between gap-2">
                        <button
                            class="btn btn-sm btn-ghost"
                            on:click={
                                let on_select_date = on_select_date.clone();
                                move |_| on_select_date.run(prev.clone())
                            }
                        >
                            "‹"
                        </button>
                        <div class="text-center">
                            <h3 class="text-xl font-bold">{formatted}</h3>
                            <span class="text-base-content/50 capitalize">{dow_name}</span>
                        </div>
                        <button
                            class="btn btn-sm btn-ghost"
                            on:click=move |_| on_select_date.run(next.clone())
                        >
                            "›"
                        </button>
                    </div>
                }
            }}

            {move || {
                let all_items = items.get();

                if loading.get() && all_items.is_empty() {
                    return view! { <LoadingSpinner/> }.into_any();
                }
                let tags = all_tags.get();
                let links = item_tag_links.get();

                if all_items.is_empty() {
                    return view! {
                        <p class="py-12 text-center text-base-content/50">
                            {move_tr!("calendar-empty-day")}
                        </p>
                    }
                    .into_any();
                }

                let hidden_lists_value = hidden_lists.get();
                let hidden_tags_value = hidden_tags.get();
                let show_completed_value = show_completed.get();

                let filtered: Vec<DateItem> = all_items
                    .into_iter()
                    .filter(|item| {
                        if hidden_lists_value.contains(&item.list_id) {
                            return false;
                        }
                        if !show_completed_value && item.completed {
                            return false;
                        }
                        if !hidden_tags_value.is_empty() {
                            let item_tags: HashSet<String> = links
                                .iter()
                                .filter(|link| link.item_id == item.id)
                                .map(|link| link.tag_id.clone())
                                .collect();
                            if hidden_tags_value
                                .iter()
                                .any(|tag_id| item_tags.contains(tag_id))
                            {
                                return false;
                            }
                        }
                        true
                    })
                    .collect();

                let mut groups: BTreeMap<(String, String), Vec<DateItem>> = BTreeMap::new();
                for item in filtered {
                    let key = (item.list_id.clone(), item.list_name.clone());
                    groups.entry(key).or_default().push(item);
                }

                view! {
                    <div class="relative">
                        {move || {
                            if loading.get() {
                                view! {
                                    <div class="pointer-events-none absolute inset-x-0 top-0 z-10 mx-auto w-fit rounded-full bg-base-200/90 px-3 py-1 text-xs text-base-content/70 shadow-sm">
                                        "Odświeżanie…"
                                    </div>
                                }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }
                        }}
                        {groups.into_iter().map(|((list_id, list_name), group_items)| {
                            let tags = tags.clone();
                            let links = links.clone();
                            let client = client.clone();
                            view! {
                                <div class="mb-4">
                                    <h4 class="mb-1 text-sm font-semibold uppercase tracking-wide text-base-content/70">
                                        <A href=format!("/lists/{list_id}") attr:class="link link-hover">
                                            {list_name}
                                        </A>
                                    </h4>
                                    {group_items.into_iter().map(|date_item| {
                                        let item_id = date_item.id.clone();
                                        let item_list_id = date_item.list_id.clone();
                                        let date_type = date_item.date_type.clone();
                                        let item: Item = date_item.into();

                                        let item_tag_ids: Vec<String> = links
                                            .iter()
                                            .filter(|link| link.item_id == item_id)
                                            .map(|link| link.tag_id.clone())
                                            .collect();

                                        let toggle_list_id = item_list_id.clone();
                                        let toggle_item_id = item_id.clone();
                                        let client_toggle = client.clone();
                                        let toast_toggle = toast.clone();
                                        let sync_week_toggle = sync_week;
                                        let selected_date_toggle = selected_date;
                                        let week_items_toggle = week_items;
                                        let items_toggle = items;
                                        let on_toggle = Callback::new(move |_id: String| {
                                            let lid = toggle_list_id.clone();
                                            let iid = toggle_item_id.clone();
                                            let client_t = client_toggle.clone();
                                            let current = items_toggle
                                                .get_untracked()
                                                .iter()
                                                .find(|item| item.id == iid)
                                                .map(|item| !item.completed);
                                            let Some(next_completed) = current else { return };
                                            let iid_for_mutation = iid.clone();
                                            let iid_for_request = iid.clone();

                                            if sync_week_toggle.get_untracked() {
                                                let date_for_week = selected_date_toggle.get_untracked();
                                                let iid_for_week = iid.clone();
                                                run_dual_optimistic_mutation(
                                                    items_toggle,
                                                    week_items_toggle,
                                                    move |items| {
                                                        let Some(item) = items
                                                            .iter_mut()
                                                            .find(|item| item.id == iid_for_mutation)
                                                        else {
                                                            return false;
                                                        };
                                                        item.completed = next_completed;
                                                        true
                                                    },
                                                    move |days| {
                                                        if let Some(day) = days
                                                            .iter_mut()
                                                            .find(|day| day.date == date_for_week)
                                                        {
                                                            if let Some(item) = day
                                                                .items
                                                                .iter_mut()
                                                                .find(|item| item.id == iid_for_week)
                                                            {
                                                                item.completed = next_completed;
                                                            }
                                                        }
                                                    },
                                                    move || async move {
                                                        let req = UpdateItemRequest {
                                                            completed: Some(next_completed),
                                                            ..Default::default()
                                                        };
                                                        api::update_item(&client_t, &lid, &iid_for_request, &req)
                                                            .await
                                                            .map(|_| ())
                                                    },
                                                    move |e| toast_toggle.push(format!("Błąd: {e}"), ToastKind::Error),
                                                );
                                            } else {
                                                run_optimistic_mutation(
                                                    items_toggle,
                                                    move |items| {
                                                        let Some(item) = items
                                                            .iter_mut()
                                                            .find(|item| item.id == iid_for_mutation)
                                                        else {
                                                            return false;
                                                        };
                                                        item.completed = next_completed;
                                                        true
                                                    },
                                                    move || async move {
                                                        let req = UpdateItemRequest {
                                                            completed: Some(next_completed),
                                                            ..Default::default()
                                                        };
                                                        api::update_item(&client_t, &lid, &iid_for_request, &req)
                                                            .await
                                                            .map(|_| ())
                                                    },
                                                    move |e| toast_toggle.push(format!("Błąd: {e}"), ToastKind::Error),
                                                );
                                            }
                                        });

                                        let delete_list_id = item_list_id.clone();
                                        let delete_item_id = item_id.clone();
                                        let client_delete = client.clone();
                                        let toast_delete = toast.clone();
                                        let sync_week_delete = sync_week;
                                        let selected_date_delete = selected_date;
                                        let week_items_delete = week_items;
                                        let items_delete = items;
                                        let on_delete = Callback::new(move |_id: String| {
                                            let lid = delete_list_id.clone();
                                            let iid = delete_item_id.clone();
                                            let client_d = client_delete.clone();
                                            let iid_for_mutation = iid.clone();
                                            let iid_for_request = iid.clone();

                                            if sync_week_delete.get_untracked() {
                                                let date_for_week = selected_date_delete.get_untracked();
                                                let iid_for_week = iid.clone();
                                                run_dual_optimistic_mutation(
                                                    items_delete,
                                                    week_items_delete,
                                                    move |items| {
                                                        let before_len = items.len();
                                                        items.retain(|item| item.id != iid_for_mutation);
                                                        items.len() != before_len
                                                    },
                                                    move |days| {
                                                        if let Some(day) = days
                                                            .iter_mut()
                                                            .find(|day| day.date == date_for_week)
                                                        {
                                                            day.items.retain(|item| item.id != iid_for_week);
                                                        }
                                                    },
                                                    move || async move {
                                                        api::delete_item(&client_d, &lid, &iid_for_request).await
                                                    },
                                                    move |e| toast_delete.push(format!("Błąd: {e}"), ToastKind::Error),
                                                );
                                            } else {
                                                run_optimistic_mutation(
                                                    items_delete,
                                                    move |items| {
                                                        let before_len = items.len();
                                                        items.retain(|item| item.id != iid_for_mutation);
                                                        items.len() != before_len
                                                    },
                                                    move || async move {
                                                        api::delete_item(&client_d, &lid, &iid_for_request).await
                                                    },
                                                    move |e| toast_delete.push(format!("Błąd: {e}"), ToastKind::Error),
                                                );
                                            }
                                        });

                                        let save_list_id = item_list_id.clone();
                                        let save_item_id = item_id.clone();
                                        let client_date = client.clone();
                                        let toast_date = toast.clone();
                                        let sync_week_date = sync_week;
                                        let selected_date_save = selected_date;
                                        let week_items_save = week_items;
                                        let items_save = items;
                                        let on_date_save = Callback::new(move |(_iid, dt, date, time): (String, String, String, Option<String>)| {
                                            let Some(field) = ItemDateField::parse(&dt) else { return; };
                                            let Some(req) = build_date_update_request(&dt, &date, time.clone()) else { return; };
                                            let lid = save_list_id.clone();
                                            let iid = save_item_id.clone();
                                            let client_ds = client_date.clone();
                                            let iid_for_mutation = iid.clone();
                                            let iid_for_request = iid.clone();
                                            let date_for_items = date.clone();
                                            let date_for_week = date.clone();
                                            let time_for_mutation = time.clone();

                                            if sync_week_date.get_untracked() {
                                                let selected_day_for_week = selected_date_save.get_untracked();
                                                let iid_for_week = iid.clone();
                                                let time_for_week = time.clone();
                                                run_dual_optimistic_mutation(
                                                    items_save,
                                                    week_items_save,
                                                    move |items| {
                                                        apply_date_change_to_date_items(
                                                            items,
                                                            &iid_for_mutation,
                                                            field,
                                                            &date_for_items,
                                                            time_for_mutation.as_deref(),
                                                        )
                                                    },
                                                    move |days| {
                                                        let _ = apply_date_change_to_day_items(
                                                            days,
                                                            &selected_day_for_week,
                                                            &iid_for_week,
                                                            field,
                                                            &date_for_week,
                                                            time_for_week.as_deref(),
                                                        );
                                                    },
                                                    move || async move {
                                                        api::update_item(&client_ds, &lid, &iid_for_request, &req)
                                                            .await
                                                            .map(|_| ())
                                                    },
                                                    move |e| toast_date.push(format!("Błąd: {e}"), ToastKind::Error),
                                                );
                                            } else {
                                                run_optimistic_mutation(
                                                    items_save,
                                                    move |items| {
                                                        apply_date_change_to_date_items(
                                                            items,
                                                            &iid_for_mutation,
                                                            field,
                                                            &date,
                                                            time_for_mutation.as_deref(),
                                                        )
                                                    },
                                                    move || async move {
                                                        api::update_item(&client_ds, &lid, &iid_for_request, &req)
                                                            .await
                                                            .map(|_| ())
                                                    },
                                                    move |e| toast_date.push(format!("Błąd: {e}"), ToastKind::Error),
                                                );
                                            }
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
                                            }
                                            .into_any()
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
                                            }
                                            .into_any()
                                        }}
                                    }).collect_view()}
                                </div>
                            }
                        }).collect_view()}
                    </div>
                }
                .into_any()
            }}
        </div>
    }
    .into_any()
}
