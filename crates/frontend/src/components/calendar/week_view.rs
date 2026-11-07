use leptos::prelude::*;

use super::item_row::CalendarItemRow;
use crate::server_fns::items::{delete_item, toggle_item};
use crate::state::invalidation::mutate_with;
use kartoteka_shared::ChangeTopic;
use kartoteka_shared::types::{CalendarWeekDay, DateItem, Item};

const DAY_NAMES: [&str; 7] = ["Pon", "Wt", "Śr", "Czw", "Pt", "Sob", "Nd"];

/// Days a week cell shows an item on: the start of any date field, matching how
/// the server groups items into [`CalendarWeekDay`]s.
fn item_days(item: &Item) -> Vec<String> {
    [&item.start_date, &item.deadline, &item.hard_deadline]
        .into_iter()
        .flatten()
        .map(|date| date.start().format("%Y-%m-%d").to_string())
        .collect()
}

/// Insert an occurrence spawned by a recurrence rule into the days of the shown
/// week it falls on (possibly none, e.g. a `2w` rule jumping past the week).
fn insert_spawned(days: &mut [CalendarWeekDay], spawned: &Item, list_name: &str, list_type: &str) {
    for date in item_days(spawned) {
        if let Some(day) = days.iter_mut().find(|day| day.date == date) {
            if day.items.iter().any(|di| di.item.id == spawned.id) {
                continue;
            }
            day.items.push(DateItem {
                item: spawned.clone(),
                list_name: list_name.to_string(),
                list_type: list_type.to_string(),
            });
        }
    }
}

#[component]
pub fn WeekView(days: Vec<CalendarWeekDay>) -> impl IntoView {
    let days_signal = RwSignal::new(days);

    view! {
        <div class="grid grid-cols-1 md:grid-cols-7 gap-2">
            {move || days_signal.get().into_iter().enumerate().map(|(dow, day)| {
                let date = day.date.clone();
                let href_day = format!("/calendar/{date}");
                let short_date = date.get(5..).map(|s| s.replacen('-', ".", 1)).unwrap_or_default();
                let day_name = DAY_NAMES[dow % 7];

                view! {
                    <div class="border border-base-300 rounded-lg p-2 min-h-24">
                        <div class="text-xs font-semibold text-center mb-2 text-base-content/60">
                            <a href=href_day class="hover:text-primary block">
                                <div>{day_name}</div>
                                <div>{short_date}</div>
                            </a>
                        </div>
                        {if day.items.is_empty() {
                            view! {
                                <div class="text-xs text-center text-base-content/30 py-2">"—"</div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="flex flex-col gap-1">
                                    {day.items.into_iter().map(|date_item| {
                                        let item_id = date_item.item.id.clone();
                                        let item_id_del = item_id.clone();
                                        let date_toggle = date.clone();
                                        let date_delete = date.clone();
                                        let list_name = date_item.list_name.clone();
                                        let list_type = date_item.list_type.clone();

                                        view! {
                                            <CalendarItemRow
                                                item_id=item_id.clone()
                                                list_id=date_item.item.list_id.clone()
                                                title=date_item.item.title.clone()
                                                completed=date_item.item.completed
                                                compact=true
                                                on_toggle=Callback::new(move |()| {
                                                    let iid = item_id.clone();
                                                    let d = date_toggle.clone();
                                                    let list_name = list_name.clone();
                                                    let list_type = list_type.clone();
                                                    mutate_with(
                                                        toggle_item(iid.clone()),
                                                        &[ChangeTopic::Items],
                                                        move |change| days_signal.update(|days| {
                                                            if let Some(day) = days.iter_mut().find(|day| day.date == d) {
                                                                if let Some(di) = day.items.iter_mut().find(|di| di.item.id == iid) {
                                                                    di.item.completed = !di.item.completed;
                                                                }
                                                            }
                                                            // A completed recurring item spawns its next
                                                            // occurrence: show it right away, without
                                                            // waiting for the topic invalidation refetch.
                                                            if let Some(spawned) = change.and_then(|change| change.spawned) {
                                                                insert_spawned(days, &spawned, &list_name, &list_type);
                                                            }
                                                        }),
                                                    );
                                                })
                                                on_delete=Callback::new(move |()| {
                                                    let iid = item_id_del.clone();
                                                    let d = date_delete.clone();
                                                    mutate_with(
                                                        delete_item(iid.clone()),
                                                        &[ChangeTopic::Items],
                                                        move |()| days_signal.update(|days| {
                                                            if let Some(day) = days.iter_mut().find(|day| day.date == d) {
                                                                day.items.retain(|di| di.item.id != iid);
                                                            }
                                                        }),
                                                    );
                                                })
                                            />
                                        }
                                    }).collect_view()}
                                </div>
                            }.into_any()
                        }}
                    </div>
                }
            }).collect_view()}
        </div>
    }
}
