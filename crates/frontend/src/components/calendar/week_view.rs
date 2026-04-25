use leptos::prelude::*;

use super::item_row::CalendarItemRow;
use crate::app::{ToastContext, ToastKind};
use crate::server_fns::items::{delete_item, toggle_item};
use kartoteka_shared::types::CalendarWeekDay;

const DAY_NAMES: [&str; 7] = ["Pon", "Wt", "Śr", "Czw", "Pt", "Sob", "Nd"];

#[component]
pub fn WeekView(days: Vec<CalendarWeekDay>) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
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
                                        let toast_t = toast;
                                        let toast_d = toast;

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
                                                    leptos::task::spawn_local(async move {
                                                        match toggle_item(iid.clone()).await {
                                                            Ok(_) => days_signal.update(|days| {
                                                                if let Some(day) = days.iter_mut().find(|day| day.date == d) {
                                                                    if let Some(di) = day.items.iter_mut().find(|di| di.item.id == iid) {
                                                                        di.item.completed = !di.item.completed;
                                                                    }
                                                                }
                                                            }),
                                                            Err(e) => toast_t.push(e.to_string(), ToastKind::Error),
                                                        }
                                                    });
                                                })
                                                on_delete=Callback::new(move |()| {
                                                    let iid = item_id_del.clone();
                                                    let d = date_delete.clone();
                                                    leptos::task::spawn_local(async move {
                                                        match delete_item(iid.clone()).await {
                                                            Ok(_) => days_signal.update(|days| {
                                                                if let Some(day) = days.iter_mut().find(|day| day.date == d) {
                                                                    day.items.retain(|di| di.item.id != iid);
                                                                }
                                                            }),
                                                            Err(e) => toast_d.push(e.to_string(), ToastKind::Error),
                                                        }
                                                    });
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
