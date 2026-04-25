use kartoteka_shared::types::TimeEntry;
use leptos::prelude::*;

use crate::app::{ToastContext, ToastKind};
use crate::server_fns::time_entries::{
    get_running_timer, get_time_summary, start_timer, stop_timer,
};

fn format_seconds(secs: i64) -> String {
    if secs == 0 {
        return "—".to_string();
    }
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}h {m:02}min")
    } else if m > 0 {
        format!("{m}min {s:02}s")
    } else {
        format!("{s}s")
    }
}

#[component]
pub fn ItemTimerWidget(item_id: Signal<String>) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let (refresh, set_refresh) = signal(0u32);

    let summary_res = Resource::new(
        move || (item_id.get(), refresh.get()),
        move |(eid, _)| get_time_summary(eid),
    );

    let running_res = Resource::new(move || refresh.get(), move |_| get_running_timer());

    let is_running_this_item = move || {
        let eid = item_id.get();
        running_res
            .get()
            .and_then(|r| r.ok())
            .flatten()
            .map(|e: TimeEntry| e.item_id.as_deref() == Some(eid.as_str()))
            .unwrap_or(false)
    };

    let is_running_other = move || {
        let eid = item_id.get();
        running_res
            .get()
            .and_then(|r| r.ok())
            .flatten()
            .map(|e: TimeEntry| e.item_id.as_deref() != Some(eid.as_str()))
            .unwrap_or(false)
    };

    let on_start = move |_: leptos::ev::MouseEvent| {
        let eid = item_id.get();
        leptos::task::spawn_local(async move {
            match start_timer(eid).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    let on_stop = move |_: leptos::ev::MouseEvent| {
        leptos::task::spawn_local(async move {
            match stop_timer().await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    view! {
        <div class="mt-6">
            <h3 class="text-sm font-semibold text-base-content/60 uppercase tracking-wide mb-3">
                "Czas"
            </h3>

            <div class="flex items-center gap-3">
                <Suspense fallback=|| view! { <span class="loading loading-dots loading-xs"></span> }>
                    {move || {
                        let total = summary_res
                            .get()
                            .and_then(|r| r.ok())
                            .map(|s| s.total_seconds)
                            .unwrap_or(0);

                        if is_running_this_item() {
                            view! {
                                <span class="text-sm font-mono text-success">{"● Trwa"}</span>
                            }
                            .into_any()
                        } else {
                            view! {
                                <span class="text-sm text-base-content/70">
                                    {format_seconds(total)}
                                </span>
                            }
                            .into_any()
                        }
                    }}
                </Suspense>

                <Suspense fallback=|| view! {}>
                    {move || {
                        if is_running_this_item() {
                            view! {
                                <button
                                    type="button"
                                    class="btn btn-sm btn-error"
                                    on:click=on_stop
                                >
                                    {"■ Stop"}
                                </button>
                            }
                            .into_any()
                        } else {
                            let label = if is_running_other() {
                                "▶ Start (zatrzyma inny)"
                            } else {
                                "▶ Start"
                            };
                            view! {
                                <button
                                    type="button"
                                    class="btn btn-sm btn-outline"
                                    on:click=on_start
                                >
                                    {label}
                                </button>
                            }
                            .into_any()
                        }
                    }}
                </Suspense>
            </div>
        </div>
    }
}
