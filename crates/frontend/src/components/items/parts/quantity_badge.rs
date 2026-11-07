use kartoteka_shared::item_display::quantity_label;
use kartoteka_shared::types::Item;
use leptos::prelude::*;

use crate::components::ui::{Chip, ChipTone};
use crate::state::user_today::use_locale;

/// Read-only quantity label such as `3/5 szt.`. Nothing when the item has no quantity.
/// The interactive stepper is `QuantityStepper`.
#[component]
pub fn QuantityBadge(#[prop(into)] item: Signal<Option<Item>>) -> impl IntoView {
    let locale = use_locale();
    let label = Memo::new(move |_| {
        item.with(|i| {
            i.as_ref().and_then(|i| {
                quantity_label(
                    i.actual_quantity,
                    i.quantity,
                    i.unit.as_deref(),
                    locale.get(),
                )
            })
        })
    });

    move || {
        label.get().map(|text| {
            view! {
                <Chip tone=ChipTone::Neutral class="text-base-content/60">
                    {text}
                </Chip>
            }
        })
    }
}
