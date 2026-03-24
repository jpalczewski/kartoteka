use leptos::prelude::*;

/// Helper: get today's date as YYYY-MM-DD
fn today_str() -> String {
    let d = js_sys::Date::new_0();
    format!("{:04}-{:02}-{:02}", d.get_full_year(), d.get_month() + 1, d.get_date())
}

/// Helper: get tomorrow's date as YYYY-MM-DD
fn tomorrow_str() -> String {
    let d = js_sys::Date::new_0();
    d.set_date(d.get_date() + 1);
    format!("{:04}-{:02}-{:02}", d.get_full_year(), d.get_month() + 1, d.get_date())
}

/// Helper: get next Monday's date as YYYY-MM-DD
fn next_monday_str() -> String {
    let d = js_sys::Date::new_0();
    let dow = d.get_day(); // 0=Sun, 1=Mon, ...
    let days_until_monday = if dow == 0 { 1 } else { (8 - dow) % 7 };
    let days_until_monday = if days_until_monday == 0 { 7 } else { days_until_monday };
    d.set_date(d.get_date() + days_until_monday);
    format!("{:04}-{:02}-{:02}", d.get_full_year(), d.get_month() + 1, d.get_date())
}

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
    let show_desc = RwSignal::new(false);
    let quantity = RwSignal::new(String::new());
    let unit = RwSignal::new("szt.".to_string());
    let due_date = RwSignal::new(String::new());
    let due_hour = RwSignal::new(Option::<u32>::None);
    let due_min = RwSignal::new(Option::<u32>::None);

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
        let dt = match (due_hour.get(), due_min.get()) {
            (Some(h), Some(m)) => Some(format!("{:02}:{:02}", h, m)),
            (Some(h), None) => Some(format!("{:02}:00", h)),
            _ => None,
        };
        title.set(String::new());
        desc.set(String::new());
        show_desc.set(false);
        quantity.set(String::new());
        unit.set("szt.".to_string());
        due_date.set(String::new());
        due_hour.set(None);
        due_min.set(None);
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
                <div class="flex gap-1">
                    <input
                        type="text"
                        class="input input-bordered flex-1"
                        placeholder="Nowy element..."
                        prop:value=title
                        on:input=move |ev| title.set(event_target_value(&ev))
                        on:keydown={let s = submit.clone(); move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Enter" { s(); }
                        }}
                    />
                    <button
                        type="button"
                        class="btn btn-ghost btn-sm btn-square self-center"
                        title="Dodaj opis"
                        on:click=move |_| show_desc.update(|v| *v = !*v)
                    >
                        {move || if show_desc.get() { "▲" } else { "📝" }}
                    </button>
                </div>

                // Description - collapsible
                <div style:display=move || if show_desc.get() { "block" } else { "none" }>
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

                // Quantity fields
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

                // Date fields with quick buttons
                {if has_due_date {
                    view! {
                        <div class="flex flex-col gap-1">
                            // Quick date buttons
                            <div class="flex gap-1 flex-wrap">
                                <button type="button"
                                    class=move || if due_date.get() == today_str() { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                                    on:click=move |_| due_date.set(today_str())
                                >"Dziś"</button>
                                <button type="button"
                                    class=move || if due_date.get() == tomorrow_str() { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                                    on:click=move |_| due_date.set(tomorrow_str())
                                >"Jutro"</button>
                                <button type="button"
                                    class=move || if due_date.get() == next_monday_str() { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                                    on:click=move |_| due_date.set(next_monday_str())
                                >"Poniedziałek"</button>
                                <input
                                    type="date"
                                    class="input input-bordered input-xs w-36"
                                    prop:value=due_date
                                    on:input=move |ev| due_date.set(event_target_value(&ev))
                                />
                            </div>
                            // Time stepper
                            <div class="flex items-center gap-1">
                                <span class="text-xs opacity-50">"Godzina:"</span>
                                <button type="button" class="btn btn-xs btn-ghost"
                                    on:click=move |_| {
                                        let h = due_hour.get().unwrap_or(12);
                                        due_hour.set(Some(if h == 0 { 23 } else { h - 1 }));
                                    }
                                >"−"</button>
                                <span class="font-mono text-sm min-w-[2ch] text-center">
                                    {move || due_hour.get().map_or("--".to_string(), |h| format!("{:02}", h))}
                                </span>
                                <button type="button" class="btn btn-xs btn-ghost"
                                    on:click=move |_| {
                                        let h = due_hour.get().unwrap_or(11);
                                        due_hour.set(Some((h + 1) % 24));
                                    }
                                >"+"</button>
                                <span class="font-mono text-sm">":"</span>
                                <button type="button" class="btn btn-xs btn-ghost"
                                    on:click=move |_| {
                                        let m = due_min.get().unwrap_or(15);
                                        due_min.set(Some(if m < 15 { 45 } else { m - 15 }));
                                    }
                                >"−"</button>
                                <span class="font-mono text-sm min-w-[2ch] text-center">
                                    {move || due_min.get().map_or("--".to_string(), |m| format!("{:02}", m))}
                                </span>
                                <button type="button" class="btn btn-xs btn-ghost"
                                    on:click=move |_| {
                                        let m = due_min.get().unwrap_or(45);
                                        due_min.set(Some((m + 15) % 60));
                                    }
                                >"+"</button>
                                {move || {
                                    if due_hour.get().is_some() || due_min.get().is_some() {
                                        view! {
                                            <button type="button" class="btn btn-xs btn-ghost opacity-50"
                                                on:click=move |_| { due_hour.set(None); due_min.set(None); }
                                            >"✕"</button>
                                        }.into_any()
                                    } else {
                                        view! {}.into_any()
                                    }
                                }}
                            </div>
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
