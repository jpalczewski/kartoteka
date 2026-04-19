use leptos::prelude::*;

fn random_hex_color() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;
    let mut hasher = DefaultHasher::new();
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos()
        .hash(&mut hasher);
    let h = hasher.finish();
    format!("#{:06x}", h & 0xFFFFFF)
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
