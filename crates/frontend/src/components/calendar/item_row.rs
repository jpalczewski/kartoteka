use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::components::A;
use reactive_stores::Store;

use crate::components::ui::{IconButton, IconButtonVariant};
use crate::state::item_actions::use_item_actions;
use crate::state::item_store::{ItemRows, row_item};
use kartoteka_shared::types::DateItem;

/// One calendar row, read from the keyed item store by `id`. Created once per id
/// and kept across refetches; every displayed field reads the store reactively.
#[component]
pub fn CalendarItemRow(
    store: Store<ItemRows<DateItem>>,
    id: String,
    /// compact=true: xs sizing, no background padding (week view)
    /// compact=false: sm sizing, padded card (day view)
    #[prop(default = false)]
    compact: bool,
) -> impl IntoView {
    let item = row_item(store, id.clone());
    let completed = move || item.with(|di| di.as_ref().is_some_and(|di| di.item.completed));
    let title = move || {
        item.with(|di| {
            di.as_ref()
                .map(|di| di.item.title.clone())
                .unwrap_or_default()
        })
    };
    let href = move || {
        item.with(|di| {
            di.as_ref()
                .map(|di| format!("/lists/{}/items/{}", di.item.list_id, di.item.id))
                .unwrap_or_default()
        })
    };

    let (row_class, checkbox_class, title_class, title_class_done) = if compact {
        (
            "flex items-start gap-1 group",
            "checkbox checkbox-xs checkbox-primary mt-0.5 shrink-0",
            "flex-1 text-xs leading-tight hover:text-primary",
            "flex-1 text-xs text-base-content/50 line-through leading-tight",
        )
    } else {
        (
            "flex items-center gap-3 p-2 bg-base-200 rounded-lg group",
            "checkbox checkbox-sm checkbox-primary",
            "flex-1 text-sm text-base-content hover:text-primary",
            "flex-1 text-sm text-base-content/50 line-through",
        )
    };

    let actions = use_item_actions();
    let id_toggle = id.clone();
    let id_delete = id;

    view! {
        <div class=row_class>
            <input
                type="checkbox"
                class=checkbox_class
                prop:checked=completed
                on:change=move |_| actions.toggle.run(id_toggle.clone())
            />
            <A
                href=href
                attr:class=move || if completed() { title_class_done } else { title_class }
            >
                {title}
            </A>
            <IconButton
                icon="x"
                label=move_tr!("common-delete")
                variant=IconButtonVariant::Danger
                class="opacity-0 group-hover:opacity-100"
                on_click=Callback::new(move |_| actions.delete.run(id_delete.clone()))
            />
        </div>
    }
}
