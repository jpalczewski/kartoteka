use kartoteka_shared::{DATE_TYPE_HARD_DEADLINE, DATE_TYPE_START, Item};
use leptos::prelude::*;

use super::date_editor::DateEditor;

/// Inline date editor section: resolves date_type to border color + initial values,
/// renders DateEditor + close button. Used by ItemRow, DateItemRow.
#[component]
pub fn InlineDateEditorSection(
    /// Which date type is being edited: "start", "deadline", or "hard_deadline"
    date_type: String,
    /// The item being edited (for initial date/time values)
    item: Item,
    /// Item ID for the save callback
    item_id: String,
    /// Called with (item_id, date_type, date_value, time_value)
    on_save: Callback<(String, String, String, Option<String>)>,
    /// Called when the close button is clicked
    on_close: Callback<()>,
    /// CSS class for the wrapper div
    #[prop(default = "pl-14 pb-2".to_string())]
    wrapper_class: String,
) -> impl IntoView {
    let (border, init_date, init_time, has_time) = match date_type.as_str() {
        x if x == DATE_TYPE_START => (
            "border-info",
            item.start_date.clone(),
            item.start_time.clone(),
            true,
        ),
        x if x == DATE_TYPE_HARD_DEADLINE => {
            ("border-error", item.hard_deadline.clone(), None, false)
        }
        _ => (
            "border-warning",
            item.deadline.clone(),
            item.deadline_time.clone(),
            true,
        ),
    };

    let id_for_save = item_id;
    let dt_for_save = date_type;

    view! {
        <div class=wrapper_class>
            <DateEditor
                border_color=border
                initial_date=init_date
                initial_time=init_time
                has_time=has_time
                on_change=Callback::new(move |(date, time): (String, Option<String>)| {
                    on_save.run((id_for_save.clone(), dt_for_save.clone(), date, time));
                })
            />
            <button type="button" class="btn btn-xs btn-ghost mt-1 opacity-50"
                on:click=move |_| on_close.run(())
            >"Zamknij"</button>
        </div>
    }
}
