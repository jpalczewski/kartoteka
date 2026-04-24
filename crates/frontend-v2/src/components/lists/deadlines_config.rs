use crate::app::{ToastContext, ToastKind};
use crate::server_fns::lists::update_feature_config;
use kartoteka_shared::FEATURE_DEADLINES;
use leptos::prelude::*;

#[component]
pub fn DeadlinesConfig(
    list_id: String,
    config: serde_json::Value,
    on_changed: Callback<()>,
) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");

    let start = RwSignal::new(
        config
            .get("has_start_date")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    );
    let deadline = RwSignal::new(
        config
            .get("has_deadline")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
    );
    let hard = RwSignal::new(
        config
            .get("has_hard_deadline")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    );

    let make_toggle = move |signal: RwSignal<bool>| {
        let lid = list_id.clone();
        move |ev: leptos::ev::Event| {
            let checked = event_target_checked(&ev);
            signal.set(checked);
            let config = serde_json::json!({
                "has_start_date": start.get_untracked(),
                "has_deadline": deadline.get_untracked(),
                "has_hard_deadline": hard.get_untracked(),
            });
            let lid2 = lid.clone();
            leptos::task::spawn_local(async move {
                match update_feature_config(lid2, FEATURE_DEADLINES.to_string(), config).await {
                    Ok(_) => on_changed.run(()),
                    Err(e) => {
                        signal.set(!checked);
                        toast.push(e.to_string(), ToastKind::Error);
                    }
                }
            });
        }
    };

    view! {
        <li class="pl-4 flex flex-col gap-1 text-xs">
            <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" class="checkbox checkbox-xs"
                    prop:checked=move || start.get()
                    on:change=make_toggle(start) />
                "Data startu"
            </label>
            <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" class="checkbox checkbox-xs"
                    prop:checked=move || deadline.get()
                    on:change=make_toggle(deadline) />
                "Termin"
            </label>
            <label class="flex items-center gap-2 cursor-pointer">
                <input type="checkbox" class="checkbox checkbox-xs"
                    prop:checked=move || hard.get()
                    on:change=make_toggle(hard) />
                "Twardy termin"
            </label>
        </li>
    }
}
