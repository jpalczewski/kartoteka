use kartoteka_shared::types::Item;
use leptos::prelude::*;
use leptos_fluent::move_tr;

use crate::components::ui::{Chip, ChipTone};

/// "In progress" marker, shown only while the item's status is `in_progress`.
#[component]
pub fn StatusChip(#[prop(into)] item: Signal<Option<Item>>) -> impl IntoView {
    let in_progress =
        Memo::new(move |_| item.with(|i| i.as_ref().is_some_and(|i| i.status == "in_progress")));
    let label = move_tr!("item-status-in-progress");

    move || {
        in_progress.get().then(|| {
            view! {
                <Chip icon="play" tone=ChipTone::Primary>
                    {label}
                </Chip>
            }
        })
    }
}
