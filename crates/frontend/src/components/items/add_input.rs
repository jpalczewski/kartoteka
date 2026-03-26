use leptos::prelude::*;

/// Input field with a submit button. Handles Enter key and button click.
/// Calls `on_submit` with the current value and clears the input.
#[component]
pub fn AddInput(
    placeholder: &'static str,
    button_label: &'static str,
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

    view! {
        <>
            <input
                type="text"
                class="input input-bordered flex-1"
                placeholder=placeholder
                prop:value=value
                on:input=move |ev| set_value.set(event_target_value(&ev))
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if ev.key() == "Enter" {
                        submit();
                    }
                }
            />
            <button class="btn btn-primary" on:click=move |_| submit()>
                {button_label}
            </button>
        </>
    }
}
