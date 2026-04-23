pub mod day;

use leptos::prelude::*;
use leptos_router::components::A;

use crate::components::calendar::week_view::WeekView;
use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::items::{get_calendar_month, get_calendar_week};

pub(super) fn add_days(date: &str, days: i64) -> String {
    use chrono::{Duration, NaiveDate};
    use std::str::FromStr;
    NaiveDate::from_str(date)
        .ok()
        .and_then(|d| {
            d.checked_add_signed(Duration::days(days))
                .map(|nd| nd.format("%Y-%m-%d").to_string())
        })
        .unwrap_or_else(|| date.to_string())
}

fn prev_month_str(ym: &str) -> String {
    let year: i32 = ym.get(..4).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let month: u32 = ym
        .get(5..7)
        .and_then(|s| s.parse().ok())
        .filter(|&m| (1..=12).contains(&m))
        .unwrap_or(1);
    if month == 1 {
        format!("{:04}-12", year - 1)
    } else {
        format!("{year:04}-{:02}", month - 1)
    }
}

fn next_month_str(ym: &str) -> String {
    let year: i32 = ym.get(..4).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let month: u32 = ym
        .get(5..7)
        .and_then(|s| s.parse().ok())
        .filter(|&m| (1..=12).contains(&m))
        .unwrap_or(12);
    if month == 12 {
        format!("{:04}-01", year + 1)
    } else {
        format!("{year:04}-{:02}", month + 1)
    }
}

fn format_month_title(year: i32, month: u32) -> String {
    const NAMES: [&str; 12] = [
        "Styczeń",
        "Luty",
        "Marzec",
        "Kwiecień",
        "Maj",
        "Czerwiec",
        "Lipiec",
        "Sierpień",
        "Wrzesień",
        "Październik",
        "Listopad",
        "Grudzień",
    ];
    format!(
        "{} {year}",
        NAMES
            .get((month as usize).saturating_sub(1))
            .copied()
            .unwrap_or("?")
    )
}

fn format_week_range_label(monday: &str) -> String {
    use chrono::{Datelike, Duration, NaiveDate};
    use std::str::FromStr;
    let Ok(mon) = NaiveDate::from_str(monday) else {
        return monday.to_string();
    };
    let sun = mon + Duration::days(6);
    format!(
        "{}.{:02} – {}.{:02}.{}",
        mon.day(),
        mon.month(),
        sun.day(),
        sun.month(),
        sun.year()
    )
}

#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    Month,
    Week,
}

#[component]
pub fn CalendarPage() -> impl IntoView {
    let (view_mode, set_view_mode) = signal(ViewMode::Month);
    let (current_ym, set_current_ym) = signal(String::new());
    let (week_start, set_week_start) = signal(String::new());

    let cal_res = Resource::new(move || current_ym.get(), get_calendar_month);
    let week_res = Resource::new(move || week_start.get(), get_calendar_week);

    Effect::new(move |_| {
        if let Some(Ok(ref cal)) = cal_res.get() {
            let resolved = cal.year_month.clone();
            if current_ym.get_untracked() != resolved {
                set_current_ym.set(resolved);
            }
        }
    });

    Effect::new(move |_| {
        if let Some(Ok(ref days)) = week_res.get() {
            if let Some(first) = days.first() {
                let monday = first.date.clone();
                if week_start.get_untracked() != monday {
                    set_week_start.set(monday);
                }
            }
        }
    });

    let on_prev = move |_: leptos::ev::MouseEvent| match view_mode.get() {
        ViewMode::Month => {
            let ym = current_ym.get();
            if !ym.is_empty() {
                set_current_ym.set(prev_month_str(&ym));
            }
        }
        ViewMode::Week => {
            let ws = week_start.get();
            if !ws.is_empty() {
                set_week_start.set(add_days(&ws, -7));
            }
        }
    };
    let on_next = move |_: leptos::ev::MouseEvent| match view_mode.get() {
        ViewMode::Month => {
            let ym = current_ym.get();
            if !ym.is_empty() {
                set_current_ym.set(next_month_str(&ym));
            }
        }
        ViewMode::Week => {
            let ws = week_start.get();
            if !ws.is_empty() {
                set_week_start.set(add_days(&ws, 7));
            }
        }
    };

    view! {
        <div class="container mx-auto max-w-5xl p-4">
            <div class="flex items-center justify-between mb-4">
                <div class="join">
                    <button
                        class=move || format!("join-item btn btn-sm {}", if view_mode.get() == ViewMode::Month { "btn-primary" } else { "btn-ghost" })
                        on:click=move |_| set_view_mode.set(ViewMode::Month)
                    >"Miesiąc"</button>
                    <button
                        class=move || format!("join-item btn btn-sm {}", if view_mode.get() == ViewMode::Week { "btn-primary" } else { "btn-ghost" })
                        on:click=move |_| set_view_mode.set(ViewMode::Week)
                    >"Tydzień"</button>
                </div>
            </div>

            {move || match view_mode.get() {
                ViewMode::Month => view! {
                    <Suspense fallback=|| view! { <LoadingSpinner/> }>
                        {move || cal_res.get().map(|result| match result {
                            Err(e) => view! { <p class="text-error">"Błąd: " {e.to_string()}</p> }.into_any(),
                            Ok(cal) => {
                                let count_map: std::collections::HashMap<String, u32> = cal.items_by_day.iter()
                                    .map(|d| (d.date.clone(), d.count)).collect();
                                let title = format_month_title(cal.year, cal.month);
                                let year = cal.year;
                                let month = cal.month;
                                let first_weekday = cal.first_weekday;
                                let days_in_month = cal.days_in_month;

                                view! {
                                    <div>
                                        <div class="flex items-center justify-between mb-4">
                                            <button class="btn btn-ghost btn-sm" on:click=on_prev>"‹"</button>
                                            <h1 class="text-xl font-bold">{title}</h1>
                                            <button class="btn btn-ghost btn-sm" on:click=on_next>"›"</button>
                                        </div>
                                        <div class="grid grid-cols-7 mb-1">
                                            {["Pon", "Wt", "Śr", "Czw", "Pt", "Sob", "Nd"].iter().map(|d| view! {
                                                <div class="text-center text-xs text-base-content/50 font-semibold py-1">{*d}</div>
                                            }).collect_view()}
                                        </div>
                                        <div class="grid grid-cols-7 gap-1">
                                            {(0..first_weekday).map(|_| view! { <div></div> }).collect_view()}
                                            {(1..=days_in_month as u32).map(|day| {
                                                let date_str = format!("{year:04}-{month:02}-{day:02}");
                                                let count = count_map.get(&date_str).copied().unwrap_or(0);
                                                view! {
                                                    <A
                                                        href=format!("/calendar/{date_str}")
                                                        attr:class="flex flex-col items-center justify-start p-1 rounded-lg hover:bg-base-200 min-h-12 cursor-pointer"
                                                    >
                                                        <span class="text-sm">{day.to_string()}</span>
                                                        {(count > 0).then(|| view! {
                                                            <span class="badge badge-primary badge-xs mt-1">{count.to_string()}</span>
                                                        })}
                                                    </A>
                                                }
                                            }).collect_view()}
                                        </div>
                                    </div>
                                }.into_any()
                            }
                        })}
                    </Suspense>
                }.into_any(),

                ViewMode::Week => view! {
                    <Suspense fallback=|| view! { <LoadingSpinner/> }>
                        {move || week_res.get().map(|result| match result {
                            Err(e) => view! { <p class="text-error">"Błąd: " {e.to_string()}</p> }.into_any(),
                            Ok(days) => {
                                let monday = days.first().map(|d| d.date.clone()).unwrap_or_default();
                                let label = format_week_range_label(&monday);
                                view! {
                                    <div>
                                        <div class="flex items-center justify-between mb-4">
                                            <button class="btn btn-ghost btn-sm" on:click=on_prev>"‹"</button>
                                            <span class="text-base font-semibold">{label}</span>
                                            <button class="btn btn-ghost btn-sm" on:click=on_next>"›"</button>
                                        </div>
                                        <WeekView days=days/>
                                    </div>
                                }.into_any()
                            }
                        })}
                    </Suspense>
                }.into_any(),
            }}
        </div>
    }
}
