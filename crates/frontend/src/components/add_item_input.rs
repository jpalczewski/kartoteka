use leptos::prelude::*;

/// Form for adding a new list item.
/// Calls `on_submit` with `(title, description, quantity, unit, due_date, due_time)`.
#[component]
pub fn AddItemInput(
    on_submit: Callback<(String, Option<String>, Option<i32>, Option<String>, Option<String>, Option<String>)>,
    #[prop(default = false)] has_quantity: bool,
    #[prop(default = false)] has_due_date: bool,
) -> impl IntoView {
    let title = RwSignal::new(String::new());
    let desc = RwSignal::new(String::new());
    let quantity = RwSignal::new(String::new());
    let unit = RwSignal::new("szt.".to_string());
    let due_date = RwSignal::new(String::new());
    let due_time = RwSignal::new(String::new());

    let submit = std::rc::Rc::new(move || {
        let t = title.get();
        if t.trim().is_empty() {
            return;
        }
        let d = desc.get();
        let q: Option<i32> = quantity.get().trim().parse::<i32>().ok().map(|v| v.max(1));
        let u = if q.is_some() {
            let u_val = unit.get();
            if u_val.trim().is_empty() { None } else { Some(u_val) }
        } else {
            None
        };
        let dd = {
            let v = due_date.get();
            if v.trim().is_empty() { None } else { Some(v) }
        };
        let dt = {
            let v = due_time.get();
            if v.trim().is_empty() { None } else { Some(v) }
        };
        title.set(String::new());
        desc.set(String::new());
        quantity.set(String::new());
        unit.set("szt.".to_string());
        due_date.set(String::new());
        due_time.set(String::new());
        on_submit.run((
            t,
            if d.trim().is_empty() { None } else { Some(d) },
            q,
            u,
            dd,
            dt,
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
                                min="1"
                                class="input input-bordered input-sm w-24"
                                placeholder="Ilość"
                                prop:value=quantity
                                on:input=move |ev| {
                                    let val = event_target_value(&ev);
                                    // Clamp: reject negative and zero
                                    if let Ok(n) = val.parse::<i32>() {
                                        if n < 1 {
                                            quantity.set("1".to_string());
                                            return;
                                        }
                                    }
                                    quantity.set(val);
                                }
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
                {if has_due_date {
                    view! {
                        <div class="flex gap-1">
                            <input
                                type="date"
                                class="input input-bordered input-sm flex-1"
                                prop:value=due_date
                                on:input=move |ev| due_date.set(event_target_value(&ev))
                            />
                            <input
                                type="time"
                                class="input input-bordered input-sm w-28"
                                placeholder="Godzina"
                                prop:value=due_time
                                on:input=move |ev| due_time.set(event_target_value(&ev))
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
