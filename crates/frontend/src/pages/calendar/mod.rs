pub mod day;

use std::collections::HashSet;

use leptos::prelude::*;
use leptos_fluent::move_tr;

use crate::api;
use crate::app::{ToastContext, ToastKind};
use crate::components::calendar::ViewMode;
use crate::components::calendar::calendar_nav::CalendarNav;
use crate::components::calendar::month_grid::MonthGrid;
use crate::components::calendar::week_view::WeekView;
use crate::components::common::date_utils::{
    add_days, get_today_string, month_grid_range, next_month, parse_date, prev_month, week_range,
};
use crate::components::common::loading::LoadingSpinner;
use crate::components::filters::filter_chips::FilterChips;
use kartoteka_shared::*;

#[component]
pub fn CalendarPage() -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let today = get_today_string();

    let view_mode = RwSignal::new(ViewMode::Month);
    let current_date = RwSignal::new(today.clone());

    // Data signals
    let month_counts = RwSignal::new(Vec::<DaySummary>::new());
    let week_data = RwSignal::new(Vec::<DayItems>::new());
    let all_tags = RwSignal::new(Vec::<Tag>::new());
    let item_tag_links = RwSignal::new(Vec::<ItemTagLink>::new());
    let (loading, set_loading) = signal(true);

    // Filter signals (for week view)
    let hidden_lists = RwSignal::new(HashSet::<String>::new());
    let hidden_tags = RwSignal::new(HashSet::<String>::new());
    let show_completed = RwSignal::new(true);

    // Fetch data when current_date or view_mode changes
    let _resource = LocalResource::new(move || {
        let date = current_date.get();
        let mode = view_mode.get();
        async move {
            set_loading.set(true);

            match mode {
                ViewMode::Month => {
                    if let Some((y, m, _)) = parse_date(&date) {
                        let (from, to) = month_grid_range(y, m);
                        match api::fetch_calendar_counts(&from, &to, "all").await {
                            Ok(counts) => month_counts.set(counts),
                            Err(e) => toast.push(format!("Błąd: {e}"), ToastKind::Error),
                        }
                    }
                }
                ViewMode::Week => {
                    let (from, to) = week_range(&date);
                    match api::fetch_calendar_full(&from, &to, "all").await {
                        Ok(days) => week_data.set(days),
                        Err(e) => toast.push(format!("Błąd: {e}"), ToastKind::Error),
                    }
                    if let Ok(tags) = api::fetch_tags().await {
                        all_tags.set(tags);
                    }
                    if let Ok(links) = api::fetch_item_tag_links().await {
                        item_tag_links.set(links);
                    }
                }
            }

            set_loading.set(false);
        }
    });

    let on_prev = Callback::new(move |_: ()| {
        let date = current_date.get();
        match view_mode.get() {
            ViewMode::Month => {
                if let Some((y, m, _)) = parse_date(&date) {
                    let (ny, nm) = prev_month(y, m);
                    current_date.set(format!("{:04}-{:02}-01", ny, nm));
                }
            }
            ViewMode::Week => {
                current_date.set(add_days(&date, -7));
            }
        }
    });

    let on_next = Callback::new(move |_: ()| {
        let date = current_date.get();
        match view_mode.get() {
            ViewMode::Month => {
                if let Some((y, m, _)) = parse_date(&date) {
                    let (ny, nm) = next_month(y, m);
                    current_date.set(format!("{:04}-{:02}-01", ny, nm));
                }
            }
            ViewMode::Week => {
                current_date.set(add_days(&date, 7));
            }
        }
    });

    let today_for_view = today.clone();

    view! {
        <div class="container mx-auto max-w-5xl p-4">
            <CalendarNav
                current_date=current_date
                view_mode=view_mode
                on_prev=on_prev
                on_next=on_next
            />

            {move || {
                if loading.get() {
                    return view! { <LoadingSpinner/> }.into_any();
                }

                match view_mode.get() {
                    ViewMode::Month => {
                        let date = current_date.get();
                        if let Some((y, m, _)) = parse_date(&date) {
                            view! {
                                <MonthGrid
                                    counts=month_counts.get()
                                    year=y
                                    month=m
                                    today=today_for_view.clone()
                                />
                            }.into_any()
                        } else {
                            view! { <p>{move_tr!("calendar-parse-error")}</p> }.into_any()
                        }
                    }
                    ViewMode::Week => {
                        let days = week_data.get();
                        let tags = all_tags.get();
                        let links = item_tag_links.get();

                        // Compute filter data from week items
                        let all_items: Vec<&DateItem> = days.iter()
                            .flat_map(|d| d.items.iter())
                            .collect();

                        let mut unique_lists: Vec<(String, String)> = Vec::new();
                        let mut seen = HashSet::new();
                        for item in &all_items {
                            if seen.insert(item.list_id.clone()) {
                                unique_lists.push((item.list_id.clone(), item.list_name.clone()));
                            }
                        }

                        let item_ids: HashSet<String> = all_items.iter().map(|i| i.id.clone()).collect();
                        let relevant_tag_ids: HashSet<String> = links.iter()
                            .filter(|l| item_ids.contains(&l.item_id))
                            .map(|l| l.tag_id.clone())
                            .collect();
                        let relevant_tags: Vec<Tag> = tags.iter()
                            .filter(|t| relevant_tag_ids.contains(&t.id))
                            .cloned()
                            .collect();

                        // Apply filters to week data
                        let hl = hidden_lists.get();
                        let ht = hidden_tags.get();
                        let sc = show_completed.get();

                        let filtered_days: Vec<DayItems> = days.into_iter().map(|day| {
                            let filtered_items: Vec<DateItem> = day.items.into_iter()
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
                            DayItems { date: day.date, items: filtered_items }
                        }).collect();

                        view! {
                            <FilterChips
                                unique_lists=unique_lists
                                relevant_tags=relevant_tags
                                hidden_lists=hidden_lists
                                hidden_tags=hidden_tags
                                show_completed=show_completed
                            />
                            <WeekView
                                days=filtered_days
                                today=today_for_view.clone()
                                all_tags=tags
                                item_tag_links=links
                                items_signal=week_data
                                start_date=current_date.get()
                            />
                        }.into_any()
                    }
                }
            }}
        </div>
    }
    .into_any()
}
