use leptos::prelude::*;
use crate::server_fns::lists::update_feature_config;
use crate::app::{ToastContext, ToastKind};

#[component]
pub fn DeadlinesConfig(
    list_id: String,
    config: serde_json::Value,
    on_changed: Callback<()>,
) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");

    let has_start_date = config.get("has_start_date").and_then(|v| v.as_bool()).unwrap_or(false);
    let has_deadline = config.get("has_deadline").and_then(|v| v.as_bool()).unwrap_or(true);
    let has_hard_deadline = config.get("has_hard_deadline").and_then(|v| v.as_bool()).unwrap_or(false);

    let make_toggle = {
        let lid = list_id.clone();
        move |key: &'static str, current_start: bool, current_deadline: bool, current_hard: bool| {
            let lid2 = lid.clone();
            move |ev: leptos::ev::Event| {
                let checked = event_target_checked(&ev);
                let (s, d, h) = match key {
                    "has_start_date" => (checked, current_deadline, current_hard),
                    "has_deadline" => (current_start, checked, current_hard),
                    "has_hard_deadline" => (current_start, current_deadline, checked),
                    _ => (current_start, current_deadline, current_hard),
                };
                let lid3 = lid2.clone();
                leptos::task::spawn_local(async move {
                    let config = serde_json::json!({
                        "has_start_date": s,
                        "has_deadline": d,
                        "has_hard_deadline": h,
                    });
                    match update_feature_config(lid3, "deadlines".to_string(), config).await {
                        Ok(_) => on_changed.run(()),
                        Err(e) => toast.push(e.to_string(), ToastKind::Error),
                    }
                });
            }
        }
    };

    view! {
        <li class="pl-4 flex flex-col gap-1 text-xs">
            <label class="flex items-center gap-2 cursor-pointer">
                <input
                    type="checkbox"
                    class="checkbox checkbox-xs"
                    prop:checked=has_start_date
                    on:change=make_toggle("has_start_date", has_start_date, has_deadline, has_hard_deadline)
                />
                "Data startu"
            </label>
            <label class="flex items-center gap-2 cursor-pointer">
                <input
                    type="checkbox"
                    class="checkbox checkbox-xs"
                    prop:checked=has_deadline
                    on:change=make_toggle("has_deadline", has_start_date, has_deadline, has_hard_deadline)
                />
                "Termin"
            </label>
            <label class="flex items-center gap-2 cursor-pointer">
                <input
                    type="checkbox"
                    class="checkbox checkbox-xs"
                    prop:checked=has_hard_deadline
                    on:change=make_toggle("has_hard_deadline", has_start_date, has_deadline, has_hard_deadline)
                />
                "Twardy termin"
            </label>
        </li>
    }
}
