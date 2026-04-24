use leptos::prelude::*;

/// Today's date in the browser's local timezone as YYYY-MM-DD. On SSR falls back to empty
/// — time-sensitive quick buttons are click-only so SSR correctness is not required.
#[cfg(target_arch = "wasm32")]
fn format_date(d: &js_sys::Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date()
    )
}

pub fn today_str() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        format_date(&js_sys::Date::new_0())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        String::new()
    }
}

pub fn tomorrow_str() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let d = js_sys::Date::new_0();
        d.set_date(d.get_date() + 1);
        format_date(&d)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        String::new()
    }
}

/// Upcoming Monday. If today is Monday, advance one week.
pub fn next_monday_str() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let d = js_sys::Date::new_0();
        let dow = d.get_day(); // 0=Sun..6=Sat
        let days: u32 = if dow == 0 { 1 } else { (8 - dow) % 7 };
        let days = if days == 0 { 7 } else { days };
        d.set_date(d.get_date() + days);
        format_date(&d)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        String::new()
    }
}

/// Date + optional time editor with quick-pick buttons.
///
/// - `value` — the date signal (YYYY-MM-DD, empty string = unset).
/// - `time_value` — when `Some`, renders a time picker (HH:MM, empty = unset) below the date.
/// - `show_quick` — renders Today / Tomorrow / Monday shortcut buttons above the input.
/// - `show_clear` — renders a clear-all button that wipes both date and time.
///
/// The component only mutates the provided signals; persistence is the caller's responsibility.
#[component]
pub fn DateFieldInput(
    label: &'static str,
    value: RwSignal<String>,
    #[prop(optional)] time_value: Option<RwSignal<String>>,
    #[prop(optional)] data_testid: Option<&'static str>,
    #[prop(default = false)] show_clear: bool,
    #[prop(default = false)] show_quick: bool,
    #[prop(default = false)] large: bool,
) -> impl IntoView {
    let (gap, span_w, span_size, input_size) = if large {
        ("flex items-center gap-3", "w-32", "text-sm", "input-sm")
    } else {
        ("flex items-center gap-2", "w-28", "text-xs", "input-xs")
    };

    let btn_size = if large { "btn-sm" } else { "btn-xs" };
    let testid_str = data_testid.unwrap_or("");

    let set_date_to = move |d: String| value.set(d);
    let clear_all = move || {
        value.set(String::new());
        if let Some(t) = time_value {
            t.set(String::new());
        }
    };

    view! {
        <div class="flex flex-col gap-1">
            <div class=gap>
                <span class=format!("text-base-content/50 {span_w} {span_size}")>{label}</span>
                <input
                    type="date"
                    class=format!("input input-bordered {input_size} flex-1")
                    data-testid=testid_str
                    prop:value=move || value.get()
                    on:input=move |ev| value.set(event_target_value(&ev))
                />
                {show_clear.then(move || view! {
                    <button
                        type="button"
                        class=format!("btn btn-ghost {btn_size} text-base-content/30")
                        title="Wyczyść"
                        on:click=move |_| clear_all()
                    >"✕"</button>
                })}
            </div>

            {show_quick.then(|| view! {
                <div class="flex gap-1 flex-wrap ml-0">
                    <button
                        type="button"
                        class=move || {
                            let t = today_str();
                            format!("btn {btn_size} {}", if !t.is_empty() && value.get() == t { "btn-primary" } else { "btn-outline" })
                        }
                        on:click=move |_| set_date_to(today_str())
                    >"Dziś"</button>
                    <button
                        type="button"
                        class=move || {
                            let t = tomorrow_str();
                            format!("btn {btn_size} {}", if !t.is_empty() && value.get() == t { "btn-primary" } else { "btn-outline" })
                        }
                        on:click=move |_| set_date_to(tomorrow_str())
                    >"Jutro"</button>
                    <button
                        type="button"
                        class=move || {
                            let t = next_monday_str();
                            format!("btn {btn_size} {}", if !t.is_empty() && value.get() == t { "btn-primary" } else { "btn-outline" })
                        }
                        on:click=move |_| set_date_to(next_monday_str())
                    >"Pn"</button>
                </div>
            })}

            {time_value.map(|time| view! {
                <TimePickerRow time=time date=value btn_size=btn_size />
            })}
        </div>
    }
}

/// Inline time picker. Shows preset buttons + step controls. Hidden when no date is set,
/// since a time without a date isn't persistable as an item attribute.
#[component]
fn TimePickerRow(
    time: RwSignal<String>,
    date: RwSignal<String>,
    btn_size: &'static str,
) -> impl IntoView {
    // Parse current HH:MM into (hour, minute) components. Empty string → None.
    let parse = move || {
        let t = time.get();
        if t.is_empty() {
            return (None, None);
        }
        let (h, m) = t.split_once(':').unwrap_or((t.as_str(), "00"));
        (h.parse::<u32>().ok(), m.parse::<u32>().ok())
    };

    let set_hm = move |h: u32, m: u32| {
        time.set(format!("{:02}:{:02}", h, m));
    };

    let preset = move |h: u32, m: u32| {
        let (ch, cm) = parse();
        let active = ch == Some(h) && cm == Some(m);
        (active, h, m)
    };

    let dec_hour = move |_: leptos::ev::MouseEvent| {
        let (ch, cm) = parse();
        let h = ch.unwrap_or(12);
        set_hm(if h == 0 { 23 } else { h - 1 }, cm.unwrap_or(0));
    };
    let inc_hour = move |_: leptos::ev::MouseEvent| {
        let (ch, cm) = parse();
        let h = ch.unwrap_or(11);
        set_hm((h + 1) % 24, cm.unwrap_or(0));
    };
    let plus15m = move |_: leptos::ev::MouseEvent| {
        let (ch, cm) = parse();
        let m = cm.unwrap_or(45);
        set_hm(ch.unwrap_or(12), (m + 15) % 60);
    };
    let clear_time = move |_: leptos::ev::MouseEvent| time.set(String::new());

    view! {
        {move || {
            if date.get().is_empty() {
                return view! {}.into_any();
            }
            view! {
                <div class="flex gap-1 flex-wrap items-center">
                    {[(9u32, 0u32), (12, 0), (15, 0), (18, 0)].into_iter().map(|(h, m)| {
                        let cls = move || {
                            let (active, _, _) = preset(h, m);
                            format!("btn {btn_size} {}", if active { "btn-primary" } else { "btn-outline" })
                        };
                        let label = format!("{}:{:02}", h, m);
                        view! {
                            <button type="button" class=cls on:click=move |_| set_hm(h, m)>{label}</button>
                        }
                    }).collect::<Vec<_>>()}
                    <span class="text-base-content/30">"|"</span>
                    <button type="button" class=format!("btn btn-ghost {btn_size}") on:click=dec_hour>"−1h"</button>
                    <span class="font-mono text-sm">
                        {move || {
                            let (h, m) = parse();
                            match (h, m) {
                                (Some(h), Some(m)) => format!("{:02}:{:02}", h, m),
                                _ => "--:--".to_string(),
                            }
                        }}
                    </span>
                    <button type="button" class=format!("btn btn-ghost {btn_size}") on:click=inc_hour>"+1h"</button>
                    <button type="button" class=format!("btn btn-ghost {btn_size}") on:click=plus15m>"+15m"</button>
                    {move || (!time.get().is_empty()).then(|| view! {
                        <button
                            type="button"
                            class=format!("btn btn-ghost {btn_size} text-base-content/30")
                            on:click=clear_time
                        >"✕"</button>
                    })}
                </div>
            }.into_any()
        }}
    }
}
