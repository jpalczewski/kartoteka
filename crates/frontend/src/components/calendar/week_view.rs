use std::str::FromStr;

use chrono::{Datelike, NaiveDate};
use leptos::prelude::*;
use reactive_stores::Store;

use super::{ROW_CAPS, ROW_TESTIDS};
use crate::components::items::item_compact::{CompactLayout, ItemCompact};
use crate::state::item_store::ItemRows;
use kartoteka_shared::types::DateItem;

const DAY_NAMES: [&str; 7] = ["Pon", "Wt", "Śr", "Czw", "Pt", "Sob", "Nd"];

/// One day of the week view: its date and the ids of the items shown on it, after the
/// page's filters, in display order. The items themselves live in the item store.
#[derive(Clone, Debug, PartialEq)]
pub struct WeekDay {
    /// Date as "YYYY-MM-DD".
    pub date: String,
    pub item_ids: Vec<String>,
}

/// Index of the weekday in `DAY_NAMES` (Monday = 0).
fn day_index(date: &str) -> usize {
    NaiveDate::from_str(date)
        .map(|d| d.weekday().num_days_from_monday() as usize)
        .unwrap_or(0)
}

/// Seven day cells. Rows are read from `store` by id; the day structure comes from `days`.
#[component]
pub fn WeekView(store: Store<ItemRows<DateItem>>, days: Memo<Vec<WeekDay>>) -> impl IntoView {
    let ids_for = move |date: String| {
        days.with(|ds| {
            ds.iter()
                .find(|d| d.date == date)
                .map(|d| d.item_ids.clone())
                .unwrap_or_default()
        })
    };

    view! {
        <div class="grid grid-cols-1 md:grid-cols-7 gap-2">
            <For each=move || days.get() key=|day| day.date.clone() let:day>
                {
                    let date = StoredValue::new(day.date);
                    let href_day = format!("/calendar/{}", date.get_value());
                    let short_date = date.with_value(|d| {
                        d.get(5..).map(|s| s.replacen('-', ".", 1)).unwrap_or_default()
                    });
                    let day_name = date.with_value(|d| DAY_NAMES[day_index(d)]);
                    let has_items = move || !ids_for(date.get_value()).is_empty();

                    view! {
                        <div class="border border-base-300 rounded-lg p-2 min-h-24">
                            <div class="text-xs font-semibold text-center mb-2 text-base-content/60">
                                <a href=href_day class="hover:text-primary block">
                                    <div>{day_name}</div>
                                    <div>{short_date}</div>
                                </a>
                            </div>
                            <Show
                                when=has_items
                                fallback=|| view! {
                                    <div class="text-xs text-center text-base-content/30 py-2">"—"</div>
                                }
                            >
                                <div class="flex flex-col gap-1">
                                    <For
                                        each=move || ids_for(date.get_value())
                                        key=|id| id.clone()
                                        let:id
                                    >
                                        <ItemCompact
                                            store=store
                                            id=id
                                            caps=ROW_CAPS
                                            layout=CompactLayout::Dense
                                            testids=ROW_TESTIDS
                                        />
                                    </For>
                                </div>
                            </Show>
                        </div>
                    }
                }
            </For>
        </div>
    }
}
