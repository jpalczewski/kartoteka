use leptos::prelude::*;
use leptos_fluent::move_tr;
use wasm_bindgen_futures::spawn_local;

use crate::api;
use crate::api::preferences::put_preferences;
use kartoteka_shared::SETTING_MCP_AUTO_ENABLE_FEATURES;

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
    let auto_enable = RwSignal::new(false);

    // Load settings on mount
    leptos::task::spawn_local(async move {
        if let Ok(settings) = api::fetch_settings().await {
            let val = settings
                .get(SETTING_MCP_AUTO_ENABLE_FEATURES)
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            auto_enable.set(val);
        }
    });

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

    let i18n = expect_context::<leptos_fluent::I18n>();
    let current_lang = move || i18n.language.get().id.to_string();

    let on_lang_change = move |ev: web_sys::Event| {
        let value = event_target_value(&ev);
        let langs = i18n.languages;
        if let Some(lang) = langs.iter().find(|l| l.id.to_string() == value) {
            i18n.language.set(lang);
            let value_clone = value.clone();
            spawn_local(async move {
                let _ = put_preferences(&value_clone).await;
            });
        }
    };

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-4">{move_tr!("settings-title")}</h2>

            <div class="card bg-base-200 border border-base-300 mb-4">
                <div class="card-body">
                    <h3 class="card-title text-lg">{move_tr!("settings-mcp-section-title")}</h3>
                    <p class="text-sm text-base-content/70 mb-2">
                        {move_tr!("settings-mcp-description")}
                    </p>
                    <div class="flex gap-2 items-center">
                        <code class="bg-base-300 rounded px-3 py-2 text-sm flex-1 break-all">
                            {mcp_url.clone()}
                        </code>
                        <button
                            class="btn btn-sm btn-outline"
                            on:click=on_copy
                        >
                            {move || if copied.get() {
                                move_tr!("settings-mcp-copied").get()
                            } else {
                                move_tr!("settings-mcp-copy").get()
                            }}
                        </button>
                    </div>
                </div>
            </div>

            <div class="card bg-base-200 border border-base-300 mb-4">
                <div class="card-body">
                    <h3 class="card-title text-lg">{move_tr!("settings-language-section-title")}</h3>
                    <div class="form-control">
                        <label class="label">{move_tr!("settings-language-label")}</label>
                        <select class="select select-bordered" on:change=on_lang_change>
                            <option value="en" selected=move || current_lang() == "en">"English"</option>
                            <option value="pl" selected=move || current_lang() == "pl">"Polski"</option>
                        </select>
                    </div>
                </div>
            </div>

            <div class="card bg-base-200 border border-base-300 mb-4">
                <div class="card-body">
                    <h3 class="card-title text-lg">"Zachowanie AI"</h3>
                    <label class="label cursor-pointer justify-start gap-4">
                        <input
                            type="checkbox"
                            class="toggle toggle-sm"
                            prop:checked=auto_enable
                            on:change=move |ev| {
                                let checked = event_target_checked(&ev);
                                auto_enable.set(checked);
                                leptos::task::spawn_local(async move {
                                    let _ = api::upsert_setting(
                                        SETTING_MCP_AUTO_ENABLE_FEATURES,
                                        serde_json::Value::Bool(checked),
                                    ).await;
                                });
                            }
                        />
                        <div>
                            <div class="label-text font-medium">
                                "Automatycznie włączaj funkcje list"
                            </div>
                            <div class="label-text text-xs text-base-content/60">
                                "Gdy AI potrzebuje terminu lub ilości na liście bez tych funkcji, włączy je bez pytania."
                            </div>
                        </div>
                    </label>
                </div>
            </div>

            <div class="card bg-base-200 border border-base-300">
                <div class="card-body">
                    <p class="text-base-content/60">{move_tr!("settings-account-section")}</p>
                </div>
            </div>
        </div>
    }
}
