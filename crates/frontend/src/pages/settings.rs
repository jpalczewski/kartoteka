use leptos::prelude::*;

use crate::api;

#[component]
pub fn McpRedirect() -> impl IntoView {
    let url = format!("{}/mcp", api::auth_base());
    if let Some(window) = web_sys::window() {
        let _ = window.location().set_href(&url);
    }
    view! { <></> }
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    let mcp_url = format!("{}/mcp", api::auth_base());
    let copied = RwSignal::new(false);

    let mcp_url_copy = mcp_url.clone();
    let on_copy = move |_| {
        let url = mcp_url_copy.clone();
        if let Some(window) = web_sys::window() {
            let _ = window.navigator().clipboard().write_text(&url);
            copied.set(true);
            leptos::task::spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(2000).await;
                copied.set(false);
            });
        }
    };

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-4">"Ustawienia"</h2>

            <div class="card bg-base-200 border border-base-300 mb-4">
                <div class="card-body">
                    <h3 class="card-title text-lg">"Claude / MCP"</h3>
                    <p class="text-sm text-base-content/70 mb-2">
                        "Wklej ten URL w konfiguracji Claude Code jako MCP server:"
                    </p>
                    <div class="flex gap-2 items-center">
                        <code class="bg-base-300 rounded px-3 py-2 text-sm flex-1 break-all">
                            {mcp_url.clone()}
                        </code>
                        <button
                            class="btn btn-sm btn-outline"
                            on:click=on_copy
                        >
                            {move || if copied.get() { "Skopiowano!" } else { "Kopiuj" }}
                        </button>
                    </div>
                </div>
            </div>

            <div class="card bg-base-200 border border-base-300">
                <div class="card-body">
                    <p class="text-base-content/60">"Ustawienia konta"</p>
                </div>
            </div>
        </div>
    }
}
