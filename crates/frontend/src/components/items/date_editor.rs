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

fn next_monday_str() -> String {
    let d = js_sys::Date::new_0();
    let dow = d.get_day();
    let days = if dow == 0 { 1 } else { (8 - dow) % 7 };
    let days = if days == 0 { 7 } else { days };
    d.set_date(d.get_date() + days);
    format!(
        "{:04}-{:02}-{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date()
    )
}

/// Inline date+time editor. Used for both creating and editing items.
/// `on_change` fires immediately on every change (date or time).
/// `on_clear` fires when user removes the date entirely.
#[component]
pub fn DateEditor(
    border_color: &'static str,
    #[prop(into)] initial_date: Option<String>,
    #[prop(into)] initial_time: Option<String>,
    #[prop(default = true)] has_time: bool,
    /// Fires (date, time) on every change. date="" means cleared.
    on_change: Callback<(String, Option<String>)>,
) -> impl IntoView {
    let date_signal = RwSignal::new(initial_date.unwrap_or_default());
    let hour = RwSignal::new(Option::<u32>::None);
    let min = RwSignal::new(Option::<u32>::None);

    // Parse initial time "HH:MM" into hour/min signals
    if let Some(ref t) = initial_time {
        if let Some((h, m)) = t.split_once(':') {
            hour.set(h.parse().ok());
            min.set(m.parse().ok());
        }
    }

    let fire_change = move || {
        let d = date_signal.get();
        let t = match (hour.get(), min.get()) {
            (Some(h), Some(m)) => Some(format!("{:02}:{:02}", h, m)),
            (Some(h), None) => Some(format!("{:02}:00", h)),
            _ => None,
        };
        on_change.run((d, t));
    };

    let set_date = move |d: String| {
        date_signal.set(d);
        fire_change();
    };

    let set_time = move |h: u32, m: u32| {
        hour.set(Some(h));
        min.set(Some(m));
        fire_change();
    };

    view! {
        <div class=format!("flex flex-col gap-1 pl-2 border-l-2 {border_color}")>
            // Quick date buttons + date picker
            <div class="flex gap-1 flex-wrap items-center">
                <button type="button"
                    class=move || if date_signal.get() == today_str() { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                    on:click=move |_| set_date(today_str())
                >"Dzi\u{015B}"</button>
                <button type="button"
                    class=move || if date_signal.get() == tomorrow_str() { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                    on:click=move |_| set_date(tomorrow_str())
                >"Jutro"</button>
                <button type="button"
                    class=move || if date_signal.get() == next_monday_str() { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                    on:click=move |_| set_date(next_monday_str())
                >"Pn"</button>
                <input
                    type="date"
                    class="input input-bordered input-xs w-36"
                    prop:value=date_signal
                    on:input=move |ev| set_date(event_target_value(&ev))
                />
                // Clear button
                {move || {
                    if !date_signal.get().is_empty() {
                        view! {
                            <button type="button" class="btn btn-xs btn-ghost opacity-50"
                                on:click=move |_| {
                                    date_signal.set(String::new());
                                    hour.set(None);
                                    min.set(None);
                                    on_change.run((String::new(), None));
                                }
                            >"\u{1F5D1} Usu\u{0144}"</button>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }
                }}
            </div>
            // Time — only after date is selected
            {move || {
                if has_time && !date_signal.get().is_empty() {
                    view! {
                        <div class="flex gap-1 flex-wrap items-center">
                            <button type="button"
                                class=move || if hour.get() == Some(9) && min.get() == Some(0) { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                                on:click=move |_| set_time(9, 0)
                            >"9:00"</button>
                            <button type="button"
                                class=move || if hour.get() == Some(12) && min.get() == Some(0) { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                                on:click=move |_| set_time(12, 0)
                            >"12:00"</button>
                            <button type="button"
                                class=move || if hour.get() == Some(15) && min.get() == Some(0) { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                                on:click=move |_| set_time(15, 0)
                            >"15:00"</button>
                            <button type="button"
                                class=move || if hour.get() == Some(18) && min.get() == Some(0) { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                                on:click=move |_| set_time(18, 0)
                            >"18:00"</button>
                            <span class="text-base-content/30">"|"</span>
                            <button type="button" class="btn btn-xs btn-ghost"
                                on:click=move |_| {
                                    let h = hour.get().unwrap_or(12);
                                    hour.set(Some(if h == 0 { 23 } else { h - 1 }));
                                    fire_change();
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
                                    fire_change();
                                }
                            >"+"</button>
                            <button type="button" class="btn btn-xs btn-ghost"
                                on:click=move |_| {
                                    let m = min.get().unwrap_or(45);
                                    min.set(Some((m + 15) % 60));
                                    fire_change();
                                }
                            >"+15m"</button>
                            {move || {
                                if hour.get().is_some() || min.get().is_some() {
                                    view! {
                                        <button type="button" class="btn btn-xs btn-ghost opacity-50"
                                            on:click=move |_| { hour.set(None); min.set(None); fire_change(); }
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
