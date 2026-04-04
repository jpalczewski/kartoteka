pub mod day;

use std::collections::{HashMap, HashSet};

use chrono::Datelike;
use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};

use crate::api;
use crate::api::client::GlooClient;
use crate::app::{ToastContext, ToastKind};
use crate::components::calendar::ViewMode;
use crate::components::calendar::calendar_nav::CalendarNav;
use crate::components::calendar::month_grid::MonthGrid;
use crate::components::calendar::week_view::WeekView;
use crate::components::common::date_utils::{
    add_days, get_today_string, month_grid_range, parse_date, week_range,
};
use crate::components::common::loading::LoadingSpinner;
use crate::components::filters::filter_chips::FilterChips;
use crate::components::tags::tag_tree::build_tag_filter_options;
use crate::state::calendar_route::{
    CalendarRouteState, calendar_href, calendar_state_from_maps, calendar_state_toggle_hidden_list,
    calendar_state_toggle_hidden_tag, calendar_state_toggle_show_completed,
    calendar_state_with_selected_date, calendar_state_with_view_mode, shift_month_clamped,
};
use crate::state::view_helpers::TagFilterOption;
use kartoteka_shared::*;

use self::day::CalendarDayPanel;

fn collect_unique_lists(items: &[DateItem]) -> Vec<(String, String)> {
    let mut unique_lists = Vec::new();
    let mut seen = HashSet::new();
    for item in items {
        if seen.insert(item.list_id.clone()) {
            unique_lists.push((item.list_id.clone(), item.list_name.clone()));
        }
    }
    unique_lists
}

fn collect_relevant_tags(
    items: &[DateItem],
    tags: &[Tag],
    links: &[ItemTagLink],
) -> Vec<TagFilterOption> {
    let item_ids: HashSet<String> = items.iter().map(|item| item.id.clone()).collect();
    let relevant_tag_ids: HashSet<String> = links
        .iter()
        .filter(|link| item_ids.contains(&link.item_id))
        .map(|link| link.tag_id.clone())
        .collect();

    let relevant_tag_ids: Vec<String> = relevant_tag_ids.into_iter().collect();
    build_tag_filter_options(tags, &relevant_tag_ids)
}

fn filter_day_items(
    items: &[DateItem],
    links: &[ItemTagLink],
    hidden_lists: &HashSet<String>,
    hidden_tags: &HashSet<String>,
    show_completed: bool,
) -> Vec<DateItem> {
    items
        .iter()
        .filter(|item| {
            if hidden_lists.contains(&item.list_id) {
                return false;
            }
            if !show_completed && item.completed {
                return false;
            }
            if !hidden_tags.is_empty() {
                let item_tags: HashSet<String> = links
                    .iter()
                    .filter(|link| link.item_id == item.id)
                    .map(|link| link.tag_id.clone())
                    .collect();
                if hidden_tags.iter().any(|tag_id| item_tags.contains(tag_id)) {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect()
}

fn filter_week_items(
    days: &[DayItems],
    links: &[ItemTagLink],
    hidden_lists: &HashSet<String>,
    hidden_tags: &HashSet<String>,
    show_completed: bool,
) -> Vec<DayItems> {
    days.iter()
        .map(|day| DayItems {
            date: day.date.clone(),
            items: filter_day_items(&day.items, links, hidden_lists, hidden_tags, show_completed),
        })
        .collect()
}

fn same_month(left: &str, right: &str) -> bool {
    left.get(..7) == right.get(..7)
}

fn same_week(left: &str, right: &str) -> bool {
    week_range(left) == week_range(right)
}

#[component]
pub fn CalendarRootRedirect() -> impl IntoView {
    let navigate = use_navigate();
    let today = get_today_string();

    Effect::new(move |_| {
        let state = CalendarRouteState::new(today.clone());
        navigate(&calendar_href(&state), Default::default());
    });

    view! { <LoadingSpinner/> }
}

#[component]
pub fn CalendarPage() -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let client = use_context::<GlooClient>().expect("GlooClient not provided");
    let navigate = use_navigate();
    let params = use_params_map();
    let query = use_query_map();
    let today = get_today_string();

    let selected_date = RwSignal::new(today.clone());
    let anchor_date = RwSignal::new(today.clone());
    let view_mode = RwSignal::new(ViewMode::Month);
    let hidden_lists = RwSignal::new(HashSet::<String>::new());
    let hidden_tags = RwSignal::new(HashSet::<String>::new());
    let show_completed = RwSignal::new(true);

    let month_counts = RwSignal::new(Vec::<DaySummary>::new());
    let week_data = RwSignal::new(Vec::<DayItems>::new());
    let day_items = RwSignal::new(Vec::<DateItem>::new());
    let month_counts_cache = RwSignal::new(HashMap::<String, Vec<DaySummary>>::new());
    let week_data_cache = RwSignal::new(HashMap::<String, Vec<DayItems>>::new());
    let day_items_cache = RwSignal::new(HashMap::<String, Vec<DateItem>>::new());
    let all_tags = RwSignal::new(Vec::<Tag>::new());
    let item_tag_links = RwSignal::new(Vec::<ItemTagLink>::new());

    let (calendar_loading, set_calendar_loading) = signal(true);
    let (day_loading, set_day_loading) = signal(true);

    let today_for_route = today.clone();
    let route_state = Memo::new(move |_| {
        let params = params.read();
        let query = query.read();
        calendar_state_from_maps(&params, &query, &today_for_route)
    });

    Effect::new(move |_| {
        let state = route_state.get();
        let current_mode = view_mode.get_untracked();
        let current_anchor = anchor_date.get_untracked();

        if selected_date.get_untracked() != state.selected_date {
            selected_date.set(state.selected_date.clone());
        }
        let should_sync_anchor = current_mode != state.view_mode
            || current_anchor.is_empty()
            || match state.view_mode {
                ViewMode::Month => !same_month(&current_anchor, &state.selected_date),
                ViewMode::Week => !same_week(&current_anchor, &state.selected_date),
            };
        if should_sync_anchor {
            anchor_date.set(state.selected_date.clone());
        }
        if view_mode.get_untracked() != state.view_mode {
            view_mode.set(state.view_mode);
        }
        if hidden_lists.get_untracked() != state.hidden_lists {
            hidden_lists.set(state.hidden_lists.clone());
        }
        if hidden_tags.get_untracked() != state.hidden_tags {
            hidden_tags.set(state.hidden_tags.clone());
        }
        if show_completed.get_untracked() != state.show_completed {
            show_completed.set(state.show_completed);
        }
    });

    let _metadata_resource = {
        let client = client.clone();
        LocalResource::new(move || {
            let client = client.clone();
            async move {
                if let Ok(tags) = api::fetch_tags(&client).await {
                    all_tags.set(tags);
                }
                if let Ok(links) = api::fetch_item_tag_links(&client).await {
                    item_tag_links.set(links);
                }
            }
        })
    };

    let _calendar_resource = {
        let client = client.clone();
        LocalResource::new(move || {
            let date = anchor_date.get();
            let mode = view_mode.get();
            let calendar_key = match mode {
                ViewMode::Month => parse_date(&date)
                    .map(|date| format!("month-{:04}-{:02}", date.year(), date.month()))
                    .unwrap_or_else(|| "month-invalid".to_string()),
                ViewMode::Week => {
                    let (from, to) = week_range(&date);
                    format!("week-{from}-{to}")
                }
            };
            let client = client.clone();
            let month_counts_cache = month_counts_cache;
            let week_data_cache = week_data_cache;
            async move {
                match mode {
                    ViewMode::Month => {
                        if let Some(cached) = month_counts_cache
                            .get_untracked()
                            .get(&calendar_key)
                            .cloned()
                        {
                            month_counts.set(cached);
                            set_calendar_loading.set(false);
                            return;
                        }
                    }
                    ViewMode::Week => {
                        if let Some(cached) =
                            week_data_cache.get_untracked().get(&calendar_key).cloned()
                        {
                            week_data.set(cached);
                            set_calendar_loading.set(false);
                            return;
                        }
                    }
                }

                set_calendar_loading.set(true);

                match mode {
                    ViewMode::Month => {
                        if let Some(d) = parse_date(&date) {
                            let (from, to) = month_grid_range(d.year(), d.month());
                            match api::fetch_calendar_counts(&client, &from, &to, "all").await {
                                Ok(counts) => {
                                    month_counts_cache.update(|cache| {
                                        cache.insert(calendar_key.clone(), counts.clone());
                                    });
                                    month_counts.set(counts);
                                }
                                Err(e) => toast.push(format!("Błąd: {e}"), ToastKind::Error),
                            }
                        }
                    }
                    ViewMode::Week => {
                        let (from, to) = week_range(&date);
                        match api::fetch_calendar_full(&client, &from, &to, "all").await {
                            Ok(days) => {
                                week_data_cache.update(|cache| {
                                    cache.insert(calendar_key.clone(), days.clone());
                                });
                                week_data.set(days);
                            }
                            Err(e) => toast.push(format!("Błąd: {e}"), ToastKind::Error),
                        }
                    }
                }

                set_calendar_loading.set(false);
            }
        })
    };

    let _day_resource = {
        let client = client.clone();
        LocalResource::new(move || {
            let date = selected_date.get();
            let mode = view_mode.get();
            let anchor = anchor_date.get();
            let current_week = week_data.get();
            let client = client.clone();
            let day_items_cache = day_items_cache;
            async move {
                if mode == ViewMode::Week && same_week(&anchor, &date) {
                    let week_items_for_day = current_week
                        .iter()
                        .find(|day| day.date == date)
                        .map(|day| day.items.clone())
                        .unwrap_or_default();
                    day_items_cache.update(|cache| {
                        cache.insert(date.clone(), week_items_for_day.clone());
                    });
                    day_items.set(week_items_for_day);
                    set_day_loading.set(false);
                    return;
                }

                if let Some(cached) = day_items_cache.get_untracked().get(&date).cloned() {
                    day_items.set(cached);
                    set_day_loading.set(false);
                    return;
                }

                set_day_loading.set(true);
                match api::fetch_items_by_date(&client, &date, false, "all").await {
                    Ok(items) => {
                        day_items_cache.update(|cache| {
                            cache.insert(date.clone(), items.clone());
                        });
                        day_items.set(items);
                    }
                    Err(e) => toast.push(format!("Błąd: {e}"), ToastKind::Error),
                }
                set_day_loading.set(false);
            }
        })
    };

    let navigate_prev = navigate.clone();
    let on_prev = Callback::new(move |_: ()| {
        let route = route_state.get_untracked();
        let next_anchor = match route.view_mode {
            ViewMode::Month => shift_month_clamped(&anchor_date.get_untracked(), -1),
            ViewMode::Week => add_days(&anchor_date.get_untracked(), -7),
        };
        anchor_date.set(next_anchor.clone());
        selected_date.set(next_anchor.clone());
        let next_state = calendar_state_with_selected_date(&route, next_anchor);
        navigate_prev(&calendar_href(&next_state), Default::default());
    });

    let navigate_next = navigate.clone();
    let on_next = Callback::new(move |_: ()| {
        let route = route_state.get_untracked();
        let next_anchor = match route.view_mode {
            ViewMode::Month => shift_month_clamped(&anchor_date.get_untracked(), 1),
            ViewMode::Week => add_days(&anchor_date.get_untracked(), 7),
        };
        anchor_date.set(next_anchor.clone());
        selected_date.set(next_anchor.clone());
        let next_state = calendar_state_with_selected_date(&route, next_anchor);
        navigate_next(&calendar_href(&next_state), Default::default());
    });

    let navigate_select_date = navigate.clone();
    let on_select_date = Callback::new(move |date: String| {
        let route = route_state.get_untracked();
        let current_anchor = anchor_date.get_untracked();
        let should_update_anchor = match route.view_mode {
            ViewMode::Month => !same_month(&current_anchor, &date),
            ViewMode::Week => !same_week(&current_anchor, &date),
        };
        selected_date.set(date.clone());
        if should_update_anchor {
            anchor_date.set(date.clone());
        }
        let next_state = calendar_state_with_selected_date(&route, date);
        navigate_select_date(&calendar_href(&next_state), Default::default());
    });

    let navigate_view_mode = navigate.clone();
    let on_view_mode_change = Callback::new(move |mode: ViewMode| {
        let route = route_state.get_untracked();
        view_mode.set(mode);
        anchor_date.set(route.selected_date.clone());
        let next_state = calendar_state_with_view_mode(&route, mode);
        navigate_view_mode(&calendar_href(&next_state), Default::default());
    });

    let navigate_today = navigate.clone();
    let on_today = Callback::new(move |_: ()| {
        let route = route_state.get_untracked();
        let today = get_today_string();
        anchor_date.set(today.clone());
        selected_date.set(today.clone());
        let next_state = calendar_state_with_selected_date(&route, today);
        navigate_today(&calendar_href(&next_state), Default::default());
    });

    let navigate_toggle_list = navigate.clone();
    let on_toggle_list = Callback::new(move |list_id: String| {
        let route = route_state.get_untracked();
        let next_state = calendar_state_toggle_hidden_list(&route, &list_id);
        navigate_toggle_list(&calendar_href(&next_state), Default::default());
    });

    let navigate_toggle_tag = navigate.clone();
    let on_toggle_tag = Callback::new(move |tag_id: String| {
        let route = route_state.get_untracked();
        let next_state = calendar_state_toggle_hidden_tag(&route, &tag_id);
        navigate_toggle_tag(&calendar_href(&next_state), Default::default());
    });

    let navigate_toggle_show_completed = navigate.clone();
    let on_toggle_show_completed = Callback::new(move |_: ()| {
        let route = route_state.get_untracked();
        let next_state = calendar_state_toggle_show_completed(&route);
        navigate_toggle_show_completed(&calendar_href(&next_state), Default::default());
    });

    let today_for_view = today.clone();

    view! {
        <div class="container mx-auto max-w-5xl p-4">
            <CalendarNav
                anchor_date=anchor_date
                view_mode=view_mode
                on_prev=on_prev
                on_next=on_next
                on_view_mode_change=on_view_mode_change
                on_today=on_today
            />

            {move || {
                match view_mode.get() {
                    ViewMode::Month => {
                        let date = anchor_date.get();
                        if let Some(d) = parse_date(&date) {
                            view! {
                                <div class="relative">
                                    <MonthGrid
                                        counts=month_counts.get()
                                        year=d.year()
                                        month=d.month()
                                        today=today_for_view.clone()
                                        selected_date=selected_date.get()
                                        on_select=on_select_date
                                    />
                                    {move || {
                                        if calendar_loading.get() {
                                            view! {
                                                <div class="pointer-events-none absolute inset-0 rounded-xl bg-base-100/50"></div>
                                            }.into_any()
                                        } else {
                                            view! { <></> }.into_any()
                                        }
                                    }}
                                </div>
                            }
                            .into_any()
                        } else {
                            view! { <p>{move_tr!("calendar-parse-error")}</p> }.into_any()
                        }
                    }
                    ViewMode::Week => {
                        let links = item_tag_links.get();
                        let filtered_days = filter_week_items(
                            &week_data.get(),
                            &links,
                            &hidden_lists.get(),
                            &hidden_tags.get(),
                            show_completed.get(),
                        );

                        view! {
                            <div class="relative">
                                <WeekView
                                    days=filtered_days
                                    today=today_for_view.clone()
                                    all_tags=all_tags.get()
                                    item_tag_links=links
                                    items_signal=week_data
                                    start_date=anchor_date.get()
                                    selected_date=selected_date.get()
                                    on_select=on_select_date
                                />
                                {move || {
                                    if calendar_loading.get() {
                                        view! {
                                            <div class="pointer-events-none absolute inset-0 rounded-xl bg-base-100/50"></div>
                                        }.into_any()
                                    } else {
                                        view! { <></> }.into_any()
                                    }
                                }}
                            </div>
                        }
                        .into_any()
                    }
                }
            }}

            {move || {
                let base_items = match view_mode.get() {
                    ViewMode::Month => day_items.get(),
                    ViewMode::Week => week_data
                        .get()
                        .into_iter()
                        .flat_map(|day| day.items)
                        .collect(),
                };

                let tags = all_tags.get();
                let links = item_tag_links.get();

                view! {
                    <FilterChips
                        unique_lists=collect_unique_lists(&base_items)
                        relevant_tags=collect_relevant_tags(&base_items, &tags, &links)
                        hidden_lists=hidden_lists.get()
                        hidden_tags=hidden_tags.get()
                        show_completed=show_completed.get()
                        on_toggle_list=on_toggle_list
                        on_toggle_tag=on_toggle_tag
                        on_toggle_show_completed=on_toggle_show_completed
                    />
                }
            }}

            <CalendarDayPanel
                selected_date=selected_date
                items=day_items
                loading=day_loading
                all_tags=all_tags
                item_tag_links=item_tag_links
                hidden_lists=hidden_lists
                hidden_tags=hidden_tags
                show_completed=show_completed
                on_select_date=on_select_date
                week_items=week_data
                sync_week=Signal::derive(move || view_mode.get() == ViewMode::Week)
            />
        </div>
    }
    .into_any()
}
