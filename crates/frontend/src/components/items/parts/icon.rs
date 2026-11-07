use kartoteka_shared::types::Item;
use leptos::prelude::*;

use crate::components::ui::EntityIcon;

/// The item's own icon (`lucide:` or `emoji:` value). Nothing when the item has none; items
/// have no fallback icon. A malformed stored value renders nothing too.
#[component]
pub fn ItemIcon(
    #[prop(into)] item: Signal<Option<Item>>,
    #[prop(default = 16)] size: usize,
) -> impl IntoView {
    let raw = Memo::new(move |_| item.with(|i| i.as_ref().and_then(|i| i.icon.clone())));

    move || {
        raw.get().map(|raw| {
            view! { <EntityIcon raw=Some(raw) fallback="" size=size /> }
        })
    }
}
