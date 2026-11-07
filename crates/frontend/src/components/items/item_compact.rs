//! `Compact` item view: the dense, editor-free row of the cross-list pages (Today, All,
//! calendar day/week, tag detail).
//!
//! What the four per-page rows had in common: a keyed-store row read by id, an optional
//! completion checkbox, the title as a link to the item, and an optional delete button.
//! What differed is now configuration:
//!
//! | page         | checkbox | dates | quantity | delete | layout |
//! |--------------|----------|-------|----------|--------|--------|
//! | Today        | yes      | yes   | yes      | yes    | Card   |
//! | All          | yes      | yes   | yes      | no     | Card   |
//! | calendar day | yes      | no    | no       | yes    | Card   |
//! | calendar week| yes      | no    | no       | yes    | Dense  |
//! | tag detail   | no       | no    | no       | no     | Plain  |
//!
//! Notes and log lists have no completion concept: the checkbox is hidden for them and the
//! item's description is shown under the title instead (see [`shows_checkbox`]).

use leptos::prelude::*;
use leptos_fluent::move_tr;
use reactive_stores::Store;

use crate::components::items::item_view::ItemCaps;
use crate::components::items::parts::{
    DateChips, ItemCheckbox, ItemIcon, ItemTitle, QuantityBadge, RecurrenceChip, StatusChip,
};
use crate::components::ui::{IconButton, IconButtonVariant};
use crate::state::item_actions::use_item_actions;
use crate::state::item_store::{ItemEntry, ItemRows, item_signal, row_item};

/// Box around a compact row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompactLayout {
    /// Padded card on a tinted background (Today, All, calendar day).
    #[default]
    Card,
    /// Extra small, no background (calendar week cell).
    Dense,
    /// Borderless line with a hover tint (inside a plain list, e.g. tag detail).
    Plain,
}

impl CompactLayout {
    const fn row_class(self) -> &'static str {
        match self {
            Self::Card => "flex items-center gap-3 p-2 bg-base-200 rounded-lg group",
            Self::Dense => "flex items-start gap-1 group",
            Self::Plain => {
                "flex items-center gap-2 rounded-md px-1 py-1 hover:bg-base-200/50 group"
            }
        }
    }

    const fn checkbox_class(self) -> &'static str {
        match self {
            Self::Dense => "mt-0.5 shrink-0 flex",
            Self::Card | Self::Plain => "shrink-0 flex",
        }
    }
}

/// `data-testid`s of the parts of a row, for pages whose e2e tests address them.
#[derive(Clone, Copy, Debug, Default)]
pub struct CompactTestIds {
    pub row: Option<&'static str>,
    pub toggle: Option<&'static str>,
    pub title: Option<&'static str>,
    pub description: Option<&'static str>,
}

/// Notes and log lists have no completion concept, so their items must not render a checkbox.
fn shows_checkbox(list_type: &str) -> bool {
    !matches!(list_type, "notes" | "log")
}

/// One compact row. Created once per item id and kept across refetches; every displayed
/// field reads the keyed store, so only what changed re-renders.
#[component]
pub fn ItemCompact<T: ItemEntry>(
    store: Store<ItemRows<T>>,
    #[prop(into)] id: String,
    caps: ItemCaps,
    #[prop(optional)] layout: CompactLayout,
    #[prop(optional)] testids: CompactTestIds,
) -> impl IntoView {
    let row = row_item(store, id.clone());
    let item = item_signal(row);
    let id = StoredValue::new(id);
    let actions = use_item_actions();

    let dense = layout == CompactLayout::Dense;
    let checkbox = Memo::new(move |_| {
        caps.checkbox
            && row.with(|r| {
                r.as_ref()
                    .is_some_and(|r| r.list_type().is_none_or(shows_checkbox))
            })
    });
    // Shown under the title of notes and log items, in place of the checkbox.
    let description = Memo::new(move |_| {
        row.with(|r| {
            r.as_ref()
                .filter(|r| !r.list_type().is_none_or(shows_checkbox))
                .and_then(|r| r.item().description.clone())
                .filter(|d| !d.trim().is_empty())
        })
    });

    view! {
        <div class=layout.row_class() data-testid=testids.row>
            {move || checkbox.get().then(|| view! {
                <span class=layout.checkbox_class()>
                    <ItemCheckbox
                        item=item
                        compact=dense
                        testid=testids.toggle
                        on_toggle=actions.toggle
                    />
                </span>
            })}
            <ItemIcon item=item size=14/>
            <div class="flex-1 min-w-0">
                <ItemTitle
                    item=item
                    compact=!dense
                    dense=dense
                    testid=testids.title
                />
                {move || description.get().map(|text| view! {
                    <p
                        class="text-xs text-base-content/60 line-clamp-2"
                        data-testid=testids.description
                    >
                        {text}
                    </p>
                })}
            </div>
            {(caps.dates || caps.quantity).then(|| view! {
                <div class="flex flex-wrap items-center justify-end gap-1 shrink-0">
                    <StatusChip item=item/>
                    {caps.quantity.then(|| view! { <QuantityBadge item=item/> })}
                    {caps.dates.then(|| view! {
                        <RecurrenceChip item=item/>
                        <DateChips item=item compact=true/>
                    })}
                </div>
            })}
            {caps.delete.then(|| view! {
                <IconButton
                    icon="x"
                    label=move_tr!("common-delete")
                    variant=IconButtonVariant::Danger
                    class="opacity-0 group-hover:opacity-100"
                    on_click=Callback::new(move |_| actions.delete.run(id.get_value()))
                />
            })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkbox_only_for_completable_list_types() {
        assert!(shows_checkbox("checklist"));
        assert!(shows_checkbox("shopping"));
        assert!(shows_checkbox("schedule"));
        assert!(!shows_checkbox("log"));
        assert!(!shows_checkbox("notes"));
    }
}
