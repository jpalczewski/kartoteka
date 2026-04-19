use leptos::prelude::*;

fn random_hex_color() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static S: AtomicU64 = AtomicU64::new(0x517c_c1b7_2722_0a95);
    let mut v = S.fetch_add(0x9e37_79b9_7f4a_7c15, Ordering::Relaxed);
    v ^= v << 13;
    v ^= v >> 7;
    v ^= v << 17;
    S.store(v, Ordering::Relaxed);
    format!("#{:06x}", v & 0xFF_FFFF)
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
