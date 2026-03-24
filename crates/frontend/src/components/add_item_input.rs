use leptos::prelude::*;

/// Two-field form for adding a new list item (title + optional description + optional quantity/unit).
/// Calls `on_submit` with `(title, description, quantity, unit)` and clears all inputs.
#[component]
pub fn AddItemInput(
    on_submit: Callback<(String, Option<String>, Option<i32>, Option<String>)>,
    #[prop(default = false)] has_quantity: bool,
) -> impl IntoView {
    let title = RwSignal::new(String::new());
    let desc = RwSignal::new(String::new());
    let quantity = RwSignal::new(String::new());
    let unit = RwSignal::new("szt.".to_string());

    let submit = std::rc::Rc::new(move || {
        let t = title.get();
        if t.trim().is_empty() {
            return;
        }
        let d = desc.get();
        let q: Option<i32> = quantity.get().trim().parse().ok();
        let u = if q.is_some() {
            let u_val = unit.get();
            if u_val.trim().is_empty() {
                None
            } else {
                Some(u_val)
            }
        } else {
            None
        };
        title.set(String::new());
        desc.set(String::new());
        quantity.set(String::new());
        unit.set("szt.".to_string());
        on_submit.run((
            t,
            if d.trim().is_empty() { None } else { Some(d) },
            q,
            u,
        ));
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
                {if has_quantity {
                    view! {
                        <div class="flex gap-1">
                            <input
                                type="number"
                                class="input input-bordered input-sm w-24"
                                placeholder="Ilość"
                                prop:value=quantity
                                on:input=move |ev| quantity.set(event_target_value(&ev))
                                on:keydown={let s = submit.clone(); move |ev: web_sys::KeyboardEvent| {
                                    if ev.key() == "Enter" { s(); }
                                }}
                            />
                            <input
                                type="text"
                                class="input input-bordered input-sm w-20"
                                placeholder="jedn."
                                prop:value=unit
                                on:input=move |ev| unit.set(event_target_value(&ev))
                                on:keydown={let s = submit.clone(); move |ev: web_sys::KeyboardEvent| {
                                    if ev.key() == "Enter" { s(); }
                                }}
                            />
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}
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
