use leptos::prelude::*;

/// Click-on-color component. Shows color dot, click opens picker + random button.
#[component]
pub fn EditableColor(
    color: String,
    on_save: Callback<String>,
) -> impl IntoView {
    let (editing, set_editing) = signal(false);
    let current_color = RwSignal::new(color.clone());

    let random_color = move || {
        let r = (js_sys::Math::random() * 255.0) as u8;
        let g = (js_sys::Math::random() * 255.0) as u8;
        let b = (js_sys::Math::random() * 255.0) as u8;
        format!("#{:02x}{:02x}{:02x}", r, g, b)
    };

    view! {
        <div class="relative inline-flex items-center">
            <span
                class="inline-block w-5 h-5 rounded-full cursor-pointer border-2 border-transparent hover:border-primary transition-colors"
                style=move || format!("background: {}", current_color.get())
                title="Kliknij aby zmienić kolor"
                on:click=move |_| set_editing.update(|v| *v = !*v)
            ></span>
            {move || {
                if editing.get() {
                    view! {
                        <div class="absolute left-0 top-full mt-1 flex items-center gap-1 bg-base-200 border border-base-300 rounded-lg p-1.5 shadow-lg z-50">
                            <input
                                type="color"
                                class="w-8 h-8 rounded cursor-pointer border-0 p-0"
                                prop:value=move || current_color.get()
                                on:input=move |ev| {
                                    let c = event_target_value(&ev);
                                    current_color.set(c.clone());
                                    on_save.run(c);
                                }
                            />
                            <button
                                class="btn btn-ghost btn-xs"
                                title="Losowy kolor"
                                on:click=move |_| {
                                    let c = random_color();
                                    current_color.set(c.clone());
                                    on_save.run(c);
                                }
                            >"🎲"</button>
                            <button
                                class="btn btn-ghost btn-xs"
                                on:click=move |_| set_editing.set(false)
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
