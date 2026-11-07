//! Editors for single item fields in the expanded [`super::item_row::ItemRow`]: status,
//! recurrence and estimate. Each one reads the item from the row's store memo and saves
//! through [`use_item_actions`]. Text drafts start from the item when the editor opens, so
//! a refetch never overwrites what is being typed.

use kartoteka_shared::types::Item;
use kartoteka_shared::{estimate_input_text, parse_estimate_minutes};
use leptos::prelude::*;
use leptos_fluent::move_tr;

use super::fields::{
    EstimateField, RecurrenceFields, estimate_is_valid, mode_value, recurrence_issue,
    recurrence_request,
};
use crate::state::item_actions::use_item_actions;

/// Status select: to do / in progress / done. `done` is completing the item, so it goes
/// through the toggle like the checkbox; it is only offered on lists that can complete items.
#[component]
pub fn StatusEditor(
    item: Memo<Option<Item>>,
    id: String,
    #[prop(into)] can_complete: Signal<bool>,
) -> impl IntoView {
    let actions = use_item_actions();
    let status =
        Memo::new(move |_| item.with(|i| i.as_ref().map(|i| i.status.clone()).unwrap_or_default()));
    let on_change = move |ev| {
        let next = event_target_value(&ev);
        if next == "done" {
            if status.get_untracked() != "done" {
                actions.toggle.run(id.clone());
            }
        } else {
            actions.set_status.run((id.clone(), next));
        }
    };

    view! {
        <div class="flex items-center gap-2">
            <span class="text-base-content/50 text-xs w-28">{move_tr!("common-status")}</span>
            <select
                class="select select-bordered select-xs w-auto"
                data-testid="item-status-select"
                on:change=on_change
            >
                <option value="todo" selected=move || status.get() == "todo">
                    {move_tr!("item-status-todo")}
                </option>
                <option value="in_progress" selected=move || status.get() == "in_progress">
                    {move_tr!("item-status-in-progress")}
                </option>
                <Show when=move || can_complete.get() || status.get() == "done">
                    <option value="done" selected=move || status.get() == "done">
                        {move_tr!("item-status-done")}
                    </option>
                </Show>
            </select>
        </div>
    }
}

/// Recurrence rule and mode with a save button. A blank rule clears the recurrence.
#[component]
pub fn RecurrenceEditor(item: Memo<Option<Item>>, id: String) -> impl IntoView {
    let actions = use_item_actions();
    let current = item.get_untracked();
    let rule = RwSignal::new(
        current
            .as_ref()
            .and_then(|i| i.recurrence_rule)
            .map(|r| r.to_string())
            .unwrap_or_default(),
    );
    let mode = RwSignal::new(mode_value(current.as_ref().and_then(|i| i.recurrence_mode)));
    // The domain checks the saved dates, so a date typed in the date editor does not count yet.
    let has_date = Memo::new(move |_| {
        item.with(|i| {
            i.as_ref()
                .is_some_and(|i| i.start_date.is_some() || i.deadline.is_some())
        })
    });
    let can_save = Memo::new(move |_| rule.with(|r| recurrence_issue(r, has_date.get()).is_none()));

    view! {
        <div class="flex items-start gap-2 flex-wrap">
            <RecurrenceFields
                rule=rule
                mode=mode
                has_date=has_date
                testid="item-recurrence-input"
            />
            <button
                type="button"
                class="btn btn-xs btn-primary"
                data-testid="item-recurrence-save"
                prop:disabled=move || !can_save.get()
                on:click=move |_| {
                    let (rule, mode) = recurrence_request(&rule.get_untracked(), &mode.get_untracked());
                    actions.set_recurrence.run((id.clone(), rule, mode));
                }
            >
                {move_tr!("common-save")}
            </button>
        </div>
    }
}

/// Estimate input with a save button. A blank value clears the estimate.
#[component]
pub fn EstimateEditor(item: Memo<Option<Item>>, id: String) -> impl IntoView {
    let actions = use_item_actions();
    let value = RwSignal::new(
        item.get_untracked()
            .and_then(|i| i.estimated_duration)
            .map(estimate_input_text)
            .unwrap_or_default(),
    );
    let valid = Memo::new(move |_| value.with(|v| estimate_is_valid(v)));

    view! {
        <div class="flex items-start gap-2">
            <EstimateField value=value testid="item-estimate-input" />
            <button
                type="button"
                class="btn btn-xs btn-primary"
                data-testid="item-estimate-save"
                prop:disabled=move || !valid.get()
                on:click=move |_| {
                    let minutes = parse_estimate_minutes(&value.get_untracked());
                    actions.set_estimate.run((id.clone(), minutes));
                }
            >
                {move_tr!("common-save")}
            </button>
        </div>
    }
}
