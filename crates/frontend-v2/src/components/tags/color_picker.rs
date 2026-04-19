use leptos::prelude::*;

/// Inline color swatch input. Calls `on_change` with the new hex color when the user commits.
#[component]
pub fn TagColorPicker(
    #[prop(into)] value: Signal<String>,
    on_change: Callback<String>,
    #[prop(default = "w-10 h-10")] size_class: &'static str,
) -> impl IntoView {
    view! {
        <input
            type="color"
            class=format!("{size_class} rounded cursor-pointer border border-base-300")
            title="Zmień kolor"
            prop:value=move || value.get()
            on:change=move |ev| on_change.run(event_target_value(&ev))
        />
    }
}
