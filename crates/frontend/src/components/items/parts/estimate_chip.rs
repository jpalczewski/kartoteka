use kartoteka_shared::format_estimate_minutes;
use kartoteka_shared::types::Item;
use leptos::prelude::*;
use leptos_fluent::move_tr;

use crate::components::ui::Chip;

/// Estimated duration badge (`1h 30m`); nothing for an item without an estimate.
#[component]
pub fn EstimateChip(#[prop(into)] item: Signal<Option<Item>>) -> impl IntoView {
    let minutes = Memo::new(move |_| item.with(|i| i.as_ref().and_then(|i| i.estimated_duration)));

    move || {
        minutes.get().map(|minutes| {
            let label = format_estimate_minutes(minutes);
            // The macro's arg closure needs its own copy: it is re-run on locale changes.
            let value = label.clone();
            let title = move_tr!("item-estimate-chip", { "value" => value.clone() });
            view! {
                <Chip icon="timer" title=title.get() testid=Some("item-estimate-chip")>
                    {label}
                </Chip>
            }
        })
    }
}
