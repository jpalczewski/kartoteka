use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
fn random_hex_color() -> String {
    let r = (js_sys::Math::random() * 256.0) as u8;
    let g = (js_sys::Math::random() * 256.0) as u8;
    let b = (js_sys::Math::random() * 256.0) as u8;
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

// Only called from browser event handlers — SSR stub never runs.
#[cfg(not(target_arch = "wasm32"))]
fn random_hex_color() -> String {
    "#6366f1".to_string()
}

/// Color swatch + 🎲 randomize button. Calls `on_change` with the new hex color.
#[component]
pub fn TagColorPicker(
    #[prop(into)] value: Signal<String>,
    on_change: Callback<String>,
    #[prop(default = "w-10 h-10")] size_class: &'static str,
) -> impl IntoView {
    let on_change_clone = on_change.clone();
    view! {
        <div class="flex items-center gap-1">
            <input
                type="color"
                class=format!("{size_class} rounded cursor-pointer border border-base-300")
                title="Zmień kolor"
                prop:value=move || value.get()
                on:change=move |ev| on_change.run(event_target_value(&ev))
            />
            <button
                type="button"
                class="btn btn-ghost btn-xs"
                title="Losuj kolor"
                on:click=move |_| on_change_clone.run(random_hex_color())
            >
                "🎲"
            </button>
        </div>
    }
}
