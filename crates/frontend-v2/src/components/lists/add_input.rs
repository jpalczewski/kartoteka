use leptos::ev;
use leptos::prelude::*;

/// Text input with Enter-key + button submit. Clears on submit.
#[component]
pub fn AddInput(
    #[prop(into)] placeholder: Signal<String>,
    #[prop(into)] button_label: Signal<String>,
    on_submit: Callback<String>,
) -> impl IntoView {
    let (value, set_value) = signal(String::new());

    let submit = move || {
        let v = value.get();
        if v.trim().is_empty() {
            return;
        }
        set_value.set(String::new());
        on_submit.run(v);
    };

    let submit_click = submit.clone();

    view! {
        <div class="flex gap-2">
            <input
                type="text"
                class="input input-bordered flex-1"
                placeholder=move || placeholder.get()
                prop:value=value
                on:input=move |ev| set_value.set(event_target_value(&ev))
                on:keydown=move |ev: ev::KeyboardEvent| {
                    if ev.key() == "Enter" { submit(); }
                }
            />
            <button
                type="button"
                class="btn btn-primary"
                on:click=move |_| submit_click()
            >
                {move || button_label.get()}
            </button>
        </div>
    }
}
