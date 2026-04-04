use kartoteka_shared::Item;
use leptos::prelude::*;

use crate::components::common::date_utils::format_date_short;

fn chip_meta(item: &Item) -> Vec<(&'static str, &'static str, String)> {
    let mut chips = Vec::new();

    if let Some(date) = item.start_date.as_deref() {
        let time = item
            .start_time
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(|value| format!(" {value}"))
            .unwrap_or_default();
        chips.push((
            "badge badge-info badge-sm",
            "Start",
            format!("{}{}", format_date_short(date), time),
        ));
    }

    if let Some(date) = item.deadline.as_deref() {
        let time = item
            .deadline_time
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(|value| format!(" {value}"))
            .unwrap_or_default();
        chips.push((
            "badge badge-warning badge-sm",
            "Termin",
            format!("{}{}", format_date_short(date), time),
        ));
    }

    if let Some(date) = item.hard_deadline.as_deref() {
        chips.push((
            "badge badge-error badge-sm",
            "Twardy",
            format_date_short(date),
        ));
    }

    chips
}

#[component]
pub fn ItemDateSummary(item: Item) -> impl IntoView {
    let chips = chip_meta(&item);

    if chips.is_empty() {
        return view! {}.into_any();
    }

    view! {
        <div class="mt-2 flex flex-wrap gap-1">
            {chips
                .into_iter()
                .map(|(class, label, value)| {
                    view! {
                        <span class=class>
                            <span class="font-medium">{label}</span>
                            <span class="opacity-70">"·"</span>
                            <span>{value}</span>
                        </span>
                    }
                })
                .collect_view()}
        </div>
    }
    .into_any()
}
