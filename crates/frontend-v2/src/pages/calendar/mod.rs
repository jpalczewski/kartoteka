pub mod day;

use leptos::prelude::*;
use leptos_router::components::A;

use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::items::get_calendar_month;

fn prev_month(ym: &str) -> String {
    let year: i32 = ym.get(..4).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let month: u32 = ym.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(1);
    if month == 1 {
        format!("{:04}-12", year - 1)
    } else {
        format!("{year:04}-{:02}", month - 1)
    }
}

fn next_month(ym: &str) -> String {
    let year: i32 = ym.get(..4).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let month: u32 = ym.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(12);
    if month == 12 {
        format!("{:04}-01", year + 1)
    } else {
        format!("{year:04}-{:02}", month + 1) }
}

fn format_month_title(year: i32, month: u32) -> String {
    const NAMES: [&str; 12] = [
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December",
    ];
    let name = NAMES
        .get((month as usize).saturating_sub(1))
        .copied()
        .unwrap_or("?");
    format!("{name} {year}")
}

#[component]
pub fn CalendarPage() -> impl IntoView {
    // Empty string → server resolves to current month
    let (current_ym, set_current_ym) = signal(String::new());

    let cal_res = Resource::new(
        move || current_ym.get(),
        |ym| get_calendar_month(ym),
    );

    // After first load, keep current_ym in sync with what the server returned
    Effect::new(move |_| {
        if let Some(Ok(ref cal)) = cal_res.get() {
            let resolved = cal.year_month.clone();
            if current_ym.get_untracked() != resolved {
                set_current_ym.set(resolved);
            }
        }
    });

    let on_prev = move |_: leptos::ev::MouseEvent| {
        let ym = current_ym.get();
        if !ym.is_empty() {
            set_current_ym.set(prev_month(&ym));
        }
    };
    let on_next = move |_: leptos::ev::MouseEvent| {
        let ym = current_ym.get();
        if !ym.is_empty() {
            set_current_ym.set(next_month(&ym));
        }
    };

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || cal_res.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="text-error">"Error: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(cal) => {
                        let count_map: std::collections::HashMap<String, u32> = cal
                            .items_by_day
                            .iter()
                            .map(|d| (d.date.clone(), d.count))
                            .collect();
                        let title = format_month_title(cal.year, cal.month);
                        let year = cal.year;
                        let month = cal.month;
                        let first_weekday = cal.first_weekday;
                        let days_in_month = cal.days_in_month;

                        view! {
                            <div>
                                // Month navigation header
                                <div class="flex items-center justify-between mb-4">
                                    <button
                                        class="btn btn-ghost btn-sm"
                                        on:click=on_prev
                                    >
                                        "‹"
                                    </button>
                                    <h1 class="text-xl font-bold">{title}</h1>
                                    <button
                                        class="btn btn-ghost btn-sm"
                                        on:click=on_next
                                    >
                                        "›"
                                    </button>
                                </div>

                                // Weekday headers (Mon–Sun)
                                <div class="grid grid-cols-7 mb-1">
                                    {["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
                                        .iter()
                                        .map(|d| view! {
                                            <div class="text-center text-xs text-base-content/50 font-semibold py-1">
                                                {*d}
                                            </div>
                                        })
                                        .collect_view()}
                                </div>

                                // Day grid
                                <div class="grid grid-cols-7 gap-1">
                                    // Empty leading cells
                                    {(0..first_weekday).map(|_| view! { <div></div> }).collect_view()}

                                    // Day cells
                                    {(1..=days_in_month as u32).map(|day| {
                                        let date_str = format!("{year:04}-{month:02}-{day:02}");
                                        let count = count_map.get(&date_str).copied().unwrap_or(0);
                                        let href = format!("/calendar/{date_str}");
                                        view! {
                                            <A
                                                href=href
                                                attr:class="flex flex-col items-center justify-start p-1 rounded-lg hover:bg-base-200 min-h-12 cursor-pointer"
                                            >
                                                <span class="text-sm">{day.to_string()}</span>
                                                {if count > 0 {
                                                    view! {
                                                        <span class="badge badge-primary badge-xs mt-1">
                                                            {count.to_string()}
                                                        </span>
                                                    }.into_any()
                                                } else { ().into_any() }}
                                            </A>
                                        }
                                    }).collect_view()}
                                </div>
                            </div>
                        }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}
