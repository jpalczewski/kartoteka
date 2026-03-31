use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use leptos_use::{
    UseServiceWorkerOptions, UseServiceWorkerReturn, use_service_worker_with_options,
};

#[component]
pub fn PwaRuntime() -> impl IntoView {
    #[cfg(target_arch = "wasm32")]
    Effect::new(move |_| {
        let UseServiceWorkerReturn {
            waiting,
            skip_waiting,
            ..
        } = use_service_worker_with_options(
            UseServiceWorkerOptions::default()
                .script_url("/service-worker.js")
                .skip_waiting_message("skipWaiting"),
        );

        Effect::new(move |_| {
            if waiting.get() {
                skip_waiting();
            }
        });
    });

    view! { <></> }
}
