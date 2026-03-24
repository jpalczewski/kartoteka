use leptos::prelude::*;

/// Two-field form for adding a new list item (title + optional description).
/// Calls `on_submit` with `(title, description)` and clears both inputs.
#[component]
pub fn AddItemInput(on_submit: Callback<(String, Option<String>)>) -> impl IntoView {
    let title = RwSignal::new(String::new());
    let desc = RwSignal::new(String::new());

    let submit = std::rc::Rc::new(move || {
        let t = title.get();
        if t.trim().is_empty() {
            return;
        }
        let d = desc.get();
        title.set(String::new());
        desc.set(String::new());
        on_submit.run((t, if d.trim().is_empty() { None } else { Some(d) }));
    });

    view! {
        <div class="flex gap-2 mb-4">
            <div class="flex flex-col gap-1 flex-1">
                <input
                    type="text"
                    class="input input-bordered w-full"
                    placeholder="Nowy element..."
                    prop:value=title
                    on:input=move |ev| title.set(event_target_value(&ev))
                    on:keydown={let s = submit.clone(); move |ev: web_sys::KeyboardEvent| {
                        if ev.key() == "Enter" { s(); }
                    }}
                />
                <input
                    type="text"
                    class="input input-bordered input-sm w-full"
                    placeholder="Opis (opcjonalnie)..."
                    prop:value=desc
                    on:input=move |ev| desc.set(event_target_value(&ev))
                    on:keydown={let s = submit.clone(); move |ev: web_sys::KeyboardEvent| {
                        if ev.key() == "Enter" { s(); }
                    }}
                />
            </div>
            <button
                class="btn btn-primary self-start"
                on:click={let s = submit.clone(); move |_| s()}
            >
                "Dodaj"
            </button>
        </div>
    }
}
