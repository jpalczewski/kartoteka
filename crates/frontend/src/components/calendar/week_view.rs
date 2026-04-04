use leptos::prelude::*;

use crate::api;
use crate::api::client::GlooClient;
use crate::app::{ToastContext, ToastKind};
use crate::components::common::date_utils::{
    add_days, format_date_short, polish_day_of_week, week_range,
};
use crate::components::items::date_item_row::DateItemRow;
use crate::state::item_mutations::run_optimistic_mutation;
use kartoteka_shared::*;

#[component]
pub fn WeekView(
    days: Vec<DayItems>,
    today: String,
    all_tags: Vec<Tag>,
    item_tag_links: Vec<ItemTagLink>,
    items_signal: RwSignal<Vec<DayItems>>,
    start_date: String,
    selected_date: String,
    on_select: Callback<String>,
) -> impl IntoView {
    let client = use_context::<GlooClient>().expect("GlooClient not provided");
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let (monday, _sunday) = week_range(&start_date);

    // Build 7 days starting from monday
    let week_dates: Vec<String> = (0..7).map(|i| add_days(&monday, i)).collect();

    view! {
        <div class="grid grid-cols-1 md:grid-cols-7 gap-2">
            {week_dates.into_iter().enumerate().map(|(dow, date)| {
                let is_today = date == today;
                let is_selected = date == selected_date;
                let day_items: Vec<DateItem> = days.iter()
                    .find(|d| d.date == date)
                    .map(|d| d.items.clone())
                    .unwrap_or_default();

                let col_class = if is_selected {
                    "border border-primary rounded-lg p-2 bg-primary/10 shadow-sm"
                } else if is_today {
                    "border border-primary rounded-lg p-2 bg-primary/5"
                } else {
                    "border border-base-300 rounded-lg p-2"
                };

                let tags = all_tags.clone();
                let links = item_tag_links.clone();
                let on_select = on_select.clone();
                let date_for_click = date.clone();

                view! {
                    <div class=col_class>
                        <button
                            class="mb-2 w-full rounded-md px-2 py-1 text-center text-xs font-semibold text-base-content/60 hover:bg-base-200"
                            on:click=move |_| on_select.run(date_for_click.clone())
                        >
                            <div>{polish_day_of_week(dow as u32)}</div>
                            <div>{format_date_short(&date)}</div>
                        </button>

                        {if day_items.is_empty() {
                            view! {
                                <div class="text-xs text-center text-base-content/30 py-2">"—"</div>
                            }.into_any()
                        } else {
                            let items_view = day_items.into_iter().map(|date_item| {
                                let item_id = date_item.id.clone();
                                let item_list_id = date_item.list_id.clone();
                                let item: Item = date_item.into();

                                let item_tag_ids: Vec<String> = links.iter()
                                    .filter(|l| l.item_id == item_id)
                                    .map(|l| l.tag_id.clone())
                                    .collect();

                                let toggle_list_id = item_list_id.clone();
                                let toggle_item_id = item_id.clone();
                                let target_date = date.clone();
                                let client_toggle = client.clone();
                                let toast_toggle = toast.clone();
                                let on_toggle = Callback::new(move |_id: String| {
                                    let lid = toggle_list_id.clone();
                                    let iid = toggle_item_id.clone();
                                    let td = target_date.clone();
                                    let client_t = client_toggle.clone();
                                    let new_completed = items_signal
                                        .get_untracked()
                                        .iter()
                                        .find(|d| d.date == td)
                                        .and_then(|d| d.items.iter().find(|i| i.id == iid))
                                        .map(|i| !i.completed);
                                    let Some(new_completed) = new_completed else { return };
                                    let td_for_mutation = td.clone();
                                    let iid_for_mutation = iid.clone();
                                    let iid_for_request = iid.clone();
                                    run_optimistic_mutation(
                                        items_signal,
                                        move |days| {
                                            let Some(day) = days
                                                .iter_mut()
                                                .find(|day| day.date == td_for_mutation)
                                            else {
                                                return false;
                                            };
                                            let Some(item) = day
                                                .items
                                                .iter_mut()
                                                .find(|item| item.id == iid_for_mutation)
                                            else {
                                                return false;
                                            };
                                            item.completed = new_completed;
                                            true
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
                                let delete_date = date.clone();
                                let client_delete = client.clone();
                                let toast_delete = toast.clone();
                                let on_delete = Callback::new(move |_id: String| {
                                    let lid = delete_list_id.clone();
                                    let iid = delete_item_id.clone();
                                    let dd = delete_date.clone();
                                    let client_d = client_delete.clone();
                                    let dd_for_mutation = dd.clone();
                                    let iid_for_mutation = iid.clone();
                                    let iid_for_request = iid.clone();
                                    run_optimistic_mutation(
                                        items_signal,
                                        move |days| {
                                            let Some(day) = days
                                                .iter_mut()
                                                .find(|day| day.date == dd_for_mutation)
                                            else {
                                                return false;
                                            };
                                            let before_len = day.items.len();
                                            day.items.retain(|item| item.id != iid_for_mutation);
                                            day.items.len() != before_len
                                        },
                                        move || async move { api::delete_item(&client_d, &lid, &iid_for_request).await },
                                        move |e| toast_delete.push(format!("Błąd: {e}"), ToastKind::Error),
                                    );
                                });

                                view! {
                                    <DateItemRow
                                        item=item
                                        on_toggle=on_toggle
                                        on_delete=on_delete
                                        all_tags=tags.clone()
                                        item_tag_ids=item_tag_ids
                                    />
                                }
                            }).collect_view();

                            view! { <div>{items_view}</div> }.into_any()
                        }}
                    </div>
                }
            }).collect_view()}
        </div>
    }
}
