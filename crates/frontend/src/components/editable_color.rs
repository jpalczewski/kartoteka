use leptos::prelude::*;

/// Click-on-color component. Shows color dot, click opens picker + random button.
/// Saves only when the popup is closed (not on every change).
/// Clicking outside closes the popup via a transparent overlay.
#[component]
pub fn EditableColor(
    color: String,
    on_save: Callback<String>,
) -> impl IntoView {
    let (editing, set_editing) = signal(false);
    let current_color = RwSignal::new(color.clone());
    let original_color = RwSignal::new(color.clone());

    let random_color = move || {
        let r = (js_sys::Math::random() * 255.0) as u8;
        let g = (js_sys::Math::random() * 255.0) as u8;
        let b = (js_sys::Math::random() * 255.0) as u8;
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    };

    let close_and_save = move || {
        set_editing.set(false);
        let c = current_color.get_untracked();
        if c != original_color.get_untracked() {
            original_color.set(c.clone());
            on_save.run(c);
        }
    };

    view! {
        <div class="relative inline-flex items-center">
            <span
                class="inline-block w-5 h-5 rounded-full cursor-pointer border-2 border-transparent hover:border-primary transition-colors"
                style=move || format!("background: {}", current_color.get())
                title="Kliknij aby zmienić kolor"
                on:click=move |_| {
                    if editing.get_untracked() {
                        close_and_save();
                    } else {
                        set_editing.set(true);
                    }
                }
            ></span>
            {move || {
                if editing.get() {
                    view! {
                        // Transparent overlay to catch clicks outside
                        <div
                            class="fixed inset-0 z-40"
                            on:click=move |_| close_and_save()
                        ></div>
                        <div class="absolute left-0 top-full mt-1 flex items-center gap-1 bg-base-200 border border-base-300 rounded-lg p-1.5 shadow-lg z-50">
                            <input
                                type="color"
                                class="w-8 h-8 rounded cursor-pointer border-0 p-0"
                                prop:value=move || current_color.get()
                                on:input=move |ev| {
                                    current_color.set(event_target_value(&ev));
                                }
                            />
                            <button
                                class="btn btn-ghost btn-xs"
                                title="Losowy kolor"
                                on:click=move |ev: leptos::ev::MouseEvent| {
                                    ev.stop_propagation();
                                    current_color.set(random_color());
                                }
                            >"🎲"</button>
                            <button
                                class="btn btn-ghost btn-xs"
                                on:click=move |ev: leptos::ev::MouseEvent| {
                                    ev.stop_propagation();
                                    close_and_save();
                                }
                            >"✕"</button>
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}
        </div>
    }
}
