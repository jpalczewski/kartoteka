use leptos::prelude::*;

use crate::api;
use crate::api::client::GlooClient;
use crate::components::common::date_utils::{
    add_days, format_date_short, polish_day_of_week, week_range,
};
use crate::components::items::date_item_row::DateItemRow;
use kartoteka_shared::*;

#[component]
pub fn WeekView(
    days: Vec<DayItems>,
    today: String,
    all_tags: Vec<Tag>,
    item_tag_links: Vec<ItemTagLink>,
    items_signal: RwSignal<Vec<DayItems>>,
    start_date: String,
) -> impl IntoView {
    let client = use_context::<GlooClient>().expect("GlooClient not provided");
    let (monday, _sunday) = week_range(&start_date);

    // Build 7 days starting from monday
    let week_dates: Vec<String> = (0..7).map(|i| add_days(&monday, i)).collect();

    view! {
        <div class="grid grid-cols-1 md:grid-cols-7 gap-2">
            {week_dates.into_iter().enumerate().map(|(dow, date)| {
                let is_today = date == today;
                let day_items: Vec<DateItem> = days.iter()
                    .find(|d| d.date == date)
                    .map(|d| d.items.clone())
                    .unwrap_or_default();

                let col_class = if is_today {
                    "border border-primary rounded-lg p-2 bg-primary/5"
                } else {
                    "border border-base-300 rounded-lg p-2"
                };

                let tags = all_tags.clone();
                let links = item_tag_links.clone();

                view! {
                    <div class=col_class>
                        <div class="text-xs font-semibold text-center mb-2 text-base-content/60">
                            <div>{polish_day_of_week(dow as u32)}</div>
                            <div>{format_date_short(&date)}</div>
                        </div>

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
                                let on_toggle = Callback::new(move |_id: String| {
                                    let lid = toggle_list_id.clone();
                                    let iid = toggle_item_id.clone();
                                    let td = target_date.clone();
                                    let client_t = client_toggle.clone();
                                    let previous = items_signal.get_untracked();
                                    let new_completed = previous
                                        .iter()
                                        .find(|d| d.date == td)
                                        .and_then(|d| d.items.iter().find(|i| i.id == iid))
                                        .map(|i| !i.completed);
                                    let Some(new_completed) = new_completed else { return };
                                    items_signal.update(|days| {
                                        if let Some(day) = days.iter_mut().find(|d| d.date == td) {
                                            if let Some(item) =
                                                day.items.iter_mut().find(|i| i.id == iid)
                                            {
                                                item.completed = new_completed;
                                            }
                                        }
                                    });
                                    leptos::task::spawn_local(async move {
                                        let req = UpdateItemRequest {
                                            completed: Some(new_completed),
                                            ..Default::default()
                                        };
                                        if api::update_item(&client_t, &lid, &iid, &req)
                                            .await
                                            .is_err()
                                        {
                                            items_signal.set(previous); // rollback
                                        }
                                    });
                                });

                                let delete_list_id = item_list_id.clone();
                                let delete_item_id = item_id.clone();
                                let delete_date = date.clone();
                                let client_delete = client.clone();
                                let on_delete = Callback::new(move |_id: String| {
                                    let lid = delete_list_id.clone();
                                    let iid = delete_item_id.clone();
                                    let dd = delete_date.clone();
                                    let client_d = client_delete.clone();
                                    let previous = items_signal.get_untracked();
                                    items_signal.update(|days| {
                                        if let Some(day) = days.iter_mut().find(|d| d.date == dd) {
                                            day.items.retain(|i| i.id != iid);
                                        }
                                    });
                                    leptos::task::spawn_local(async move {
                                        if api::delete_item(&client_d, &lid, &iid)
                                            .await
                                            .is_err()
                                        {
                                            items_signal.set(previous); // rollback
                                        }
                                    });
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
