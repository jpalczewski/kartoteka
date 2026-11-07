use kartoteka_shared::types::Item;
use leptos::prelude::*;

/// Completion checkbox for an item. Calls `on_toggle` with the item id.
#[component]
pub fn ItemCheckbox(
    #[prop(into)] item: Signal<Option<Item>>,
    on_toggle: Callback<String>,
    #[prop(optional)] compact: bool,
    #[prop(optional_no_strip)] testid: Option<&'static str>,
) -> impl IntoView {
    let present = Memo::new(move |_| item.with(Option::is_some));
    let size_class = if compact {
        "checkbox-xs"
    } else {
        "checkbox-sm"
    };

    move || {
        present.get().then(|| {
            view! {
                <input
                    type="checkbox"
                    class=format!("checkbox checkbox-primary {size_class}")
                    prop:checked=move || item.with(|i| i.as_ref().is_some_and(|i| i.completed))
                    aria-label=move || item.with(|i| i.as_ref().map(|i| i.title.clone()))
                    data-testid=testid
                    on:change=move |_| {
                        if let Some(id) = item.with(|i| i.as_ref().map(|i| i.id.clone())) {
                            on_toggle.run(id);
                        }
                    }
                />
            }
        })
    }
}
