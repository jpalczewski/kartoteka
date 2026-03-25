use kartoteka_shared::{Item, Tag};
use leptos::prelude::*;

use super::tag_badge::TagBadge;
use super::tag_selector::TagSelector;

pub fn get_today_string() -> String {
    let today = js_sys::Date::new_0();
    let year = today.get_full_year() as i32;
    let month = today.get_month() as u32 + 1;
    let day = today.get_date() as u32;
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn polish_month_abbr(month: u32) -> &'static str {
    match month {
        1 => "sty",
        2 => "lut",
        3 => "mar",
        4 => "kwi",
        5 => "maj",
        6 => "cze",
        7 => "lip",
        8 => "sie",
        9 => "wrz",
        10 => "paź",
        11 => "lis",
        12 => "gru",
        _ => "???",
    }
}

fn format_date_short(date_str: &str) -> String {
    // "YYYY-MM-DD" -> "DD mmm"
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return date_str.to_string();
    }
    let day: u32 = parts[2].parse().unwrap_or(0);
    let month: u32 = parts[1].parse().unwrap_or(0);
    format!("{} {}", day, polish_month_abbr(month))
}

fn days_in_month(year: i32, month: u32) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn date_to_days(date_str: &str) -> Option<i32> {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let d: i32 = parts[2].parse().ok()?;

    // Simple days-since-epoch calculation for diff purposes
    let mut total: i32 = 0;
    for yr in 0..y {
        if (yr % 4 == 0 && yr % 100 != 0) || yr % 400 == 0 {
            total += 366;
        } else {
            total += 365;
        }
    }
    for mo in 1..m {
        total += days_in_month(y, mo);
    }
    total += d;
    Some(total)
}

fn relative_date(date_str: &str, today_str: &str) -> String {
    let date_days = date_to_days(date_str);
    let today_days = date_to_days(today_str);
    match (date_days, today_days) {
        (Some(d), Some(t)) => {
            let diff = d - t;
            match diff {
                0 => "dzisiaj".to_string(),
                1 => "jutro".to_string(),
                -1 => "wczoraj".to_string(),
                n if n > 1 => format!("za {} dni", n),
                n => format!("{} dni temu", -n),
            }
        }
        _ => String::new(),
    }
}

fn current_time_hhmm() -> String {
    let now = js_sys::Date::new_0();
    format!("{:02}:{:02}", now.get_hours(), now.get_minutes())
}

pub fn is_overdue(item: &Item, today: &str) -> bool {
    if item.completed {
        return false;
    }
    match item.due_date.as_deref() {
        None => false,
        Some(d) if d < today => true,           // past date → overdue
        Some(d) if d == today => {
            // today: overdue only if time is set AND has passed
            match item.due_time.as_deref() {
                Some(t) if !t.is_empty() => t < current_time_hhmm().as_str(),
                _ => false,                      // no time → not overdue yet
            }
        }
        _ => false,                              // future → not overdue
    }
}

pub fn is_upcoming(item: &Item, today: &str) -> bool {
    !item.completed && !is_overdue(item, today)
}

pub fn sort_by_due_date(items: &mut [Item]) {
    items.sort_by(|a, b| {
        let da = a.due_date.as_deref().unwrap_or("9999-99-99");
        let db = b.due_date.as_deref().unwrap_or("9999-99-99");
        da.cmp(db)
    });
}

pub fn get_today() -> String {
    get_today_string()
}

#[component]
pub fn DateItemRow(
    item: Item,
    on_toggle: Callback<String>,
    on_delete: Callback<String>,
    #[prop(default = vec![])] all_tags: Vec<Tag>,
    #[prop(default = vec![])] item_tag_ids: Vec<String>,
    #[prop(optional)] on_tag_toggle: Option<Callback<String>>,
) -> impl IntoView {
    let id_toggle = item.id.clone();
    let id_delete = item.id.clone();
    let completed = item.completed;
    let today = get_today_string();

    let overdue = is_overdue(&item, &today);

    let row_class = if completed {
        "flex items-center gap-3 py-3 opacity-50"
    } else if overdue {
        "flex items-center gap-3 py-3 text-error"
    } else {
        "flex items-center gap-3 py-3"
    };

    let title_class = if completed {
        "flex-1 line-through text-base-content/50"
    } else {
        "flex-1"
    };

    let date_display = item.due_date.as_ref().map(|d| format_date_short(d));
    let relative = item
        .due_date
        .as_ref()
        .map(|d| relative_date(d, &today));
    let time_display = item.due_time.clone();

    let date_color = if completed {
        "text-right text-sm text-base-content/40"
    } else if overdue {
        "text-right text-sm text-error"
    } else {
        "text-right text-sm text-base-content/60"
    };

    view! {
        <div class="border-b border-base-300">
            <div class=row_class>
                <input
                    type="checkbox"
                    class="checkbox checkbox-secondary"
                    checked=completed
                    on:change=move |_| on_toggle.run(id_toggle.clone())
                />
                <span class=title_class>{item.title}</span>

                // Tag badges
                {if !item_tag_ids.is_empty() {
                    let item_tags: Vec<Tag> = all_tags.iter()
                        .filter(|t| item_tag_ids.contains(&t.id))
                        .cloned()
                        .collect();
                    view! {
                        <div class="flex flex-wrap gap-1">
                            {item_tags.into_iter().map(|t| {
                                match on_tag_toggle {
                                    Some(cb) => view! { <TagBadge tag=t on_remove=cb/> }.into_any(),
                                    None => view! { <TagBadge tag=t/> }.into_any(),
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}
                // Tag selector
                {if let Some(toggle_cb) = on_tag_toggle {
                    view! {
                        <TagSelector
                            all_tags=all_tags.clone()
                            selected_tag_ids=item_tag_ids.clone()
                            on_toggle=toggle_cb
                        />
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}

                <div class=date_color>
                    {date_display.map(|d| view! { <div class="font-medium">{d}</div> })}
                    {relative.map(|r| view! { <div class="text-xs">{r}</div> })}
                    {time_display.map(|t| view! { <div class="text-xs">{t}</div> })}
                </div>
                {
                    let confirming = RwSignal::new(false);
                    view! {
                        <button
                            type="button"
                            class=move || if confirming.get() { "btn btn-error btn-sm" } else { "btn btn-ghost btn-sm btn-square opacity-60 hover:opacity-100" }
                            on:click=move |_| {
                                if confirming.get() {
                                    on_delete.run(id_delete.clone());
                                    confirming.set(false);
                                } else {
                                    confirming.set(true);
                                    set_timeout(move || confirming.set(false), std::time::Duration::from_millis(2500));
                                }
                            }
                        >
                            {move || if confirming.get() { "Na pewno?" } else { "✕" }}
                        </button>
                    }
                }
            </div>
        </div>
    }
}
