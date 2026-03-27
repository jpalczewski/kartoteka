use crate::components::common::date_utils::format_date_short;
use kartoteka_shared::CreateItemRequest;
use leptos::prelude::*;

/// Helper: get today's date as YYYY-MM-DD
fn today_str() -> String {
    let d = js_sys::Date::new_0();
    format!(
        "{:04}-{:02}-{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date()
    )
}

/// Helper: get tomorrow's date as YYYY-MM-DD
fn tomorrow_str() -> String {
    let d = js_sys::Date::new_0();
    d.set_date(d.get_date() + 1);
    format!(
        "{:04}-{:02}-{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date()
    )
}

/// Helper: get next Monday's date as YYYY-MM-DD
fn next_monday_str() -> String {
    let d = js_sys::Date::new_0();
    let dow = d.get_day();
    let days_until_monday = if dow == 0 { 1 } else { (8 - dow) % 7 };
    let days_until_monday = if days_until_monday == 0 {
        7
    } else {
        days_until_monday
    };
    d.set_date(d.get_date() + days_until_monday);
    format!(
        "{:04}-{:02}-{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date()
    )
}

/// Expanded date section: quick date buttons + date picker + time stepper (if has_time)
fn render_date_section(
    border_color: &'static str,
    date_signal: RwSignal<String>,
    hour_signal: Option<RwSignal<Option<u32>>>,
    min_signal: Option<RwSignal<Option<u32>>>,
) -> impl IntoView {
    let has_time = hour_signal.is_some() && min_signal.is_some();

    view! {
        <div class=format!("flex flex-col gap-1 pl-2 border-l-2 {border_color}")>
            // Quick date buttons + date picker
            <div class="flex gap-1 flex-wrap items-center">
                <button type="button"
                    class=move || if date_signal.get() == today_str() { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                    on:click=move |_| date_signal.set(today_str())
                >"Dzi\u{015B}"</button>
                <button type="button"
                    class=move || if date_signal.get() == tomorrow_str() { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                    on:click=move |_| date_signal.set(tomorrow_str())
                >"Jutro"</button>
                <button type="button"
                    class=move || if date_signal.get() == next_monday_str() { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                    on:click=move |_| date_signal.set(next_monday_str())
                >"Pn"</button>
                <input
                    type="date"
                    class="input input-bordered input-xs w-36"
                    prop:value=date_signal
                    on:input=move |ev| date_signal.set(event_target_value(&ev))
                />
            </div>
            // Time stepper — only after a date is selected, and only if this type has time
            {move || {
                if has_time && !date_signal.get().is_empty() {
                    let hour = hour_signal.unwrap();
                    let min = min_signal.unwrap();
                    view! {
                        <div class="flex gap-1 flex-wrap items-center">
                            <button type="button"
                                class=move || {
                                    let h = hour.get();
                                    if h == Some(9) { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                                }
                                on:click=move |_| { hour.set(Some(9)); min.set(Some(0)); }
                            >"9:00"</button>
                            <button type="button"
                                class=move || {
                                    let h = hour.get();
                                    let m = min.get();
                                    if h == Some(12) && m == Some(0) { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                                }
                                on:click=move |_| { hour.set(Some(12)); min.set(Some(0)); }
                            >"12:00"</button>
                            <button type="button"
                                class=move || {
                                    let h = hour.get();
                                    if h == Some(15) { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                                }
                                on:click=move |_| { hour.set(Some(15)); min.set(Some(0)); }
                            >"15:00"</button>
                            <button type="button"
                                class=move || {
                                    let h = hour.get();
                                    if h == Some(18) { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                                }
                                on:click=move |_| { hour.set(Some(18)); min.set(Some(0)); }
                            >"18:00"</button>
                            <span class="text-base-content/30">"|"</span>
                            // Manual stepper
                            <button type="button" class="btn btn-xs btn-ghost"
                                on:click=move |_| {
                                    let h = hour.get().unwrap_or(12);
                                    hour.set(Some(if h == 0 { 23 } else { h - 1 }));
                                }
                            >"\u{2212}"</button>
                            <span class="font-mono text-sm min-w-[2ch] text-center">
                                {move || hour.get().map_or("--".to_string(), |h| format!("{:02}", h))}
                            </span>
                            <span class="font-mono text-sm">":"</span>
                            <span class="font-mono text-sm min-w-[2ch] text-center">
                                {move || min.get().map_or("--".to_string(), |m| format!("{:02}", m))}
                            </span>
                            <button type="button" class="btn btn-xs btn-ghost"
                                on:click=move |_| {
                                    let h = hour.get().unwrap_or(11);
                                    hour.set(Some((h + 1) % 24));
                                }
                            >"+"</button>
                            <button type="button" class="btn btn-xs btn-ghost"
                                on:click=move |_| {
                                    let m = min.get().unwrap_or(45);
                                    min.set(Some((m + 15) % 60));
                                }
                            >"+15m"</button>
                            {move || {
                                if hour.get().is_some() || min.get().is_some() {
                                    view! {
                                        <button type="button" class="btn btn-xs btn-ghost opacity-50"
                                            on:click=move |_| { hour.set(None); min.set(None); }
                                        >"\u{2715}"</button>
                                    }.into_any()
                                } else {
                                    view! {}.into_any()
                                }
                            }}
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}
        </div>
    }
}

/// Form for adding a new list item.
#[component]
pub fn AddItemInput(
    on_submit: Callback<CreateItemRequest>,
    #[prop(default = false)] has_quantity: bool,
    #[prop(default = serde_json::Value::Null)] deadlines_config: serde_json::Value,
) -> impl IntoView {
    let has_start_date = deadlines_config
        .get("has_start_date")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let has_deadline = deadlines_config
        .get("has_deadline")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let has_hard_deadline = deadlines_config
        .get("has_hard_deadline")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let has_any_date = has_start_date || has_deadline || has_hard_deadline;

    let title = RwSignal::new(String::new());
    let desc = RwSignal::new(String::new());
    let show_desc = RwSignal::new(false);
    let quantity = RwSignal::new(String::new());
    let unit = RwSignal::new("szt.".to_string());

    let start_date = RwSignal::new(String::new());
    let start_hour = RwSignal::new(Option::<u32>::None);
    let start_min = RwSignal::new(Option::<u32>::None);
    let deadline_date = RwSignal::new(String::new());
    let deadline_hour = RwSignal::new(Option::<u32>::None);
    let deadline_min = RwSignal::new(Option::<u32>::None);
    let hard_deadline_date = RwSignal::new(String::new());

    let start_expanded = RwSignal::new(false);
    let deadline_expanded = RwSignal::new(false);
    let hard_expanded = RwSignal::new(false);

    let submit = std::rc::Rc::new(move || {
        let t = title.get();
        if t.trim().is_empty() {
            return;
        }
        let d = desc.get();
        let q: Option<i32> = quantity.get().trim().parse::<i32>().ok().map(|v| v.max(1));
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

        let to_opt = |s: String| {
            if s.trim().is_empty() { None } else { Some(s) }
        };
        let time_opt = |h: Option<u32>, m: Option<u32>| match (h, m) {
            (Some(h), Some(m)) => Some(format!("{:02}:{:02}", h, m)),
            (Some(h), None) => Some(format!("{:02}:00", h)),
            _ => None,
        };

        let req = CreateItemRequest {
            title: t,
            description: if d.trim().is_empty() { None } else { Some(d) },
            quantity: q,
            unit: u,
            start_date: to_opt(start_date.get()),
            start_time: time_opt(start_hour.get(), start_min.get()),
            deadline: to_opt(deadline_date.get()),
            deadline_time: time_opt(deadline_hour.get(), deadline_min.get()),
            hard_deadline: to_opt(hard_deadline_date.get()),
        };

        title.set(String::new());
        desc.set(String::new());
        show_desc.set(false);
        quantity.set(String::new());
        unit.set("szt.".to_string());
        start_date.set(String::new());
        start_hour.set(None);
        start_min.set(None);
        deadline_date.set(String::new());
        deadline_hour.set(None);
        deadline_min.set(None);
        hard_deadline_date.set(String::new());
        start_expanded.set(false);
        deadline_expanded.set(false);
        hard_expanded.set(false);

        on_submit.run(req);
    });

    /// Render a date chip button
    fn chip(
        label_empty: &'static str,
        icon: &'static str,
        date_signal: RwSignal<String>,
        hour_signal: Option<RwSignal<Option<u32>>>,
        min_signal: Option<RwSignal<Option<u32>>>,
        expanded: RwSignal<bool>,
        active_class: &'static str,
    ) -> impl IntoView {
        let is_active = move || !date_signal.get().is_empty() || expanded.get();
        view! {
            <button type="button"
                class=move || if is_active() { active_class } else { "btn btn-xs btn-outline btn-ghost" }
                on:click=move |_| {
                    if !date_signal.get().is_empty() {
                        date_signal.set(String::new());
                        if let Some(h) = hour_signal { h.set(None); }
                        if let Some(m) = min_signal { m.set(None); }
                        expanded.set(false);
                    } else {
                        expanded.update(|v| *v = !*v);
                    }
                }
            >
                {move || {
                    let d = date_signal.get();
                    if d.is_empty() {
                        format!("{icon} {label_empty}")
                    } else {
                        format!("{icon} {} \u{2715}", format_date_short(&d))
                    }
                }}
            </button>
        }
    }

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
                        {move || if show_desc.get() { "\u{25B2}" } else { "\u{1F4DD}" }}
                    </button>
                </div>

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

                {if has_quantity {
                    view! {
                        <div class="flex gap-1">
                            <input
                                type="number"
                                min="1"
                                class="input input-bordered input-sm w-24"
                                placeholder="Ilo\u{015B}\u{0107}"
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

                // Date chips
                {if has_any_date {
                    view! {
                        <div class="flex flex-col gap-1 mt-1">
                            <div class="flex gap-1 flex-wrap">
                                {if has_start_date {
                                    chip("Start", "\u{1F4C5}", start_date, Some(start_hour), Some(start_min), start_expanded, "btn btn-xs btn-info").into_any()
                                } else {
                                    view! {}.into_any()
                                }}
                                {if has_deadline {
                                    chip("Termin", "\u{23F0}", deadline_date, Some(deadline_hour), Some(deadline_min), deadline_expanded, "btn btn-xs btn-warning").into_any()
                                } else {
                                    view! {}.into_any()
                                }}
                                {if has_hard_deadline {
                                    chip("Twardy", "\u{1F6A8}", hard_deadline_date, None, None, hard_expanded, "btn btn-xs btn-error").into_any()
                                } else {
                                    view! {}.into_any()
                                }}
                            </div>

                            // Expanded sections
                            {move || {
                                if has_start_date && start_expanded.get() {
                                    render_date_section("border-info", start_date, Some(start_hour), Some(start_min)).into_any()
                                } else {
                                    view! {}.into_any()
                                }
                            }}
                            {move || {
                                if has_deadline && deadline_expanded.get() {
                                    render_date_section("border-warning", deadline_date, Some(deadline_hour), Some(deadline_min)).into_any()
                                } else {
                                    view! {}.into_any()
                                }
                            }}
                            {move || {
                                if has_hard_deadline && hard_expanded.get() {
                                    render_date_section("border-error", hard_deadline_date, None, None).into_any()
                                } else {
                                    view! {}.into_any()
                                }
                            }}
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
