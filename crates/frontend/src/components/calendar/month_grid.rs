use std::collections::HashMap;

use leptos::prelude::*;

use crate::components::common::date_utils::{
    add_days, day_of_week, days_in_month, polish_day_of_week,
};
use kartoteka_shared::DaySummary;

#[component]
pub fn MonthGrid(
    counts: Vec<DaySummary>,
    year: i32,
    month: u32,
    today: String,
    selected_date: String,
    on_select: Callback<String>,
) -> impl IntoView {
    // Build lookup: date -> (total, completed)
    let count_map: HashMap<String, (u32, u32)> = counts
        .into_iter()
        .map(|s| (s.date.clone(), (s.total, s.completed)))
        .collect();

    // Build grid: find first Monday before month start
    let first_day = format!("{:04}-{:02}-01", year, month);
    let first_dow = day_of_week(&first_day);
    let grid_start = add_days(&first_day, -(first_dow as i64));

    let dim = days_in_month(year, month);
    let last_day = format!("{:04}-{:02}-{:02}", year, month, dim);
    let last_dow = day_of_week(&last_day);
    let grid_end = add_days(&last_day, (6 - last_dow) as i64);

    // Generate all dates in the grid
    let mut dates: Vec<String> = Vec::new();
    let mut current = grid_start;
    loop {
        dates.push(current.clone());
        if current == grid_end {
            break;
        }
        current = add_days(&current, 1);
    }

    let month_prefix = format!("{:04}-{:02}", year, month);
    let today_for_cells = today.clone();

    view! {
        <div>
            // Day headers
            <div class="grid grid-cols-7 gap-1 mb-1">
                {(0..7u32).map(|dow| {
                    view! {
                        <div class="text-center text-xs font-semibold text-base-content/50 py-1">
                            {polish_day_of_week(dow)}
                        </div>
                    }
                }).collect_view()}
            </div>

            // Date cells
            <div class="grid grid-cols-7 gap-1">
                {dates.into_iter().map(|date| {
                    let is_current_month = date.starts_with(&month_prefix);
                    let is_today = date == today_for_cells;
                    let is_selected = date == selected_date;
                    let day_num: u32 = date.split('-').nth(2).and_then(|d| d.parse().ok()).unwrap_or(0);
                    let counts = count_map.get(&date).copied();
                    let today_cmp = today_for_cells.clone();
                    let date_cmp = date.clone();
                    let date_for_click = date.clone();
                    let on_select = on_select.clone();

                    let cell_class = move || {
                        let mut cls = "flex min-h-12 cursor-pointer flex-col items-center justify-center rounded-lg p-1 transition-colors hover:bg-base-200".to_string();
                        if !is_current_month {
                            cls.push_str(" opacity-30");
                        }
                        if is_selected {
                            cls.push_str(" bg-primary text-primary-content shadow-sm");
                        } else if is_today {
                            cls.push_str(" ring-2 ring-primary");
                        }
                        cls
                    };

                    let indicator = {
                        let is_past = date_cmp < today_cmp;
                        move || {
                            match counts {
                                None => view! { <span></span> }.into_any(),
                                Some((total, completed)) => {
                                    let color = if completed >= total {
                                        "bg-success"
                                    } else if completed > 0 {
                                        "bg-warning"
                                    } else if is_past {
                                        "bg-error"
                                    } else {
                                        "bg-base-content/30"
                                    };
                                    view! {
                                        <span class=format!("w-2 h-2 rounded-full {color}")></span>
                                    }.into_any()
                                }
                            }
                        }
                    };

                    view! {
                        <div
                            class=cell_class
                            on:click=move |_| on_select.run(date_for_click.clone())
                        >
                            <span class="text-sm">{day_num}</span>
                            {indicator}
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}
