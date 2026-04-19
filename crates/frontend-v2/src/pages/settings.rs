use leptos::prelude::*;

use crate::app::{ToastContext, ToastKind};
use crate::components::common::loading::LoadingSpinner;
use crate::context::GlobalRefresh;
use crate::server_fns::settings::{
    create_token_sf, get_settings_page_data, revoke_token_sf, set_reg_enabled, set_setting,
};
use kartoteka_shared::types::TokenCreated;

const TIMEZONES: &[&str] = &[
    "UTC",
    "Europe/Warsaw",
    "Europe/London",
    "Europe/Berlin",
    "Europe/Paris",
    "America/New_York",
    "America/Chicago",
    "America/Denver",
    "America/Los_Angeles",
    "Asia/Tokyo",
    "Asia/Shanghai",
    "Australia/Sydney",
];

#[component]
pub fn SettingsPage() -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let global_refresh = use_context::<GlobalRefresh>().expect("GlobalRefresh missing");
    let (refresh, set_refresh) = signal(0u32);

    let data_res = Resource::new(move || refresh.get(), |_| get_settings_page_data());

    let (new_token_name, set_new_token_name) = signal(String::new());
    let (created_token, set_created_token) = signal(Option::<TokenCreated>::None);

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-6">"Ustawienia"</h2>

            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || data_res.get().map(|result| match result {
                    Err(e) => view! { <p class="text-error">"Błąd: " {e.to_string()}</p> }.into_any(),
                    Ok(data) => {
                        let current_lang = data.settings.iter()
                            .find(|s| s.key == "language")
                            .map(|s| s.value.clone())
                            .unwrap_or_else(|| "en".to_string());
                        let current_tz = data.settings.iter()
                            .find(|s| s.key == "timezone")
                            .map(|s| s.value.clone())
                            .unwrap_or_else(|| "UTC".to_string());
                        let is_admin = data.is_admin;
                        let reg_enabled = data.reg_enabled;
                        let tokens = data.tokens.clone();

                        view! {
                            <div class="flex flex-col gap-6">
                                // --- Language ---
                                <div class="card bg-base-200 p-4">
                                    <h3 class="font-semibold mb-3">"Język / Language"</h3>
                                    <select
                                        class="select select-bordered w-full max-w-xs"
                                        on:change=move |ev| {
                                            let val = event_target_value(&ev);
                                            leptos::task::spawn_local(async move {
                                                match set_setting("language".to_string(), val).await {
                                                    Ok(_) => set_refresh.update(|n| *n += 1),
                                                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                                }
                                            });
                                        }
                                    >
                                        <option value="en" selected={let cl = current_lang.clone(); move || cl == "en"}>"English"</option>
                                        <option value="pl" selected=move || current_lang == "pl">"Polski"</option>
                                    </select>
                                </div>

                                // --- Timezone ---
                                <div class="card bg-base-200 p-4">
                                    <h3 class="font-semibold mb-3">"Strefa czasowa"</h3>
                                    <select
                                        class="select select-bordered w-full max-w-xs"
                                        on:change=move |ev| {
                                            let val = event_target_value(&ev);
                                            leptos::task::spawn_local(async move {
                                                match set_setting("timezone".to_string(), val).await {
                                                    Ok(_) => {
                                                        set_refresh.update(|n| *n += 1);
                                                        global_refresh.bump();
                                                    }
                                                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                                }
                                            });
                                        }
                                    >
                                        {TIMEZONES.iter().map(|tz| {
                                            let tz_val = tz.to_string();
                                            let tz_label = tz.to_string();
                                            let selected = current_tz.clone() == tz_val;
                                            view! {
                                                <option value=tz_val selected=selected>{tz_label}</option>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </select>
                                </div>

                                // --- API Tokens ---
                                <div class="card bg-base-200 p-4">
                                    <h3 class="font-semibold mb-3">"Tokeny API"</h3>

                                    {move || created_token.get().map(|tok| view! {
                                        <div class="alert alert-success mb-3">
                                            <div>
                                                <p class="font-semibold">"Skopiuj token — nie będzie widoczny ponownie:"</p>
                                                <code class="text-xs break-all block mt-1 bg-base-300 p-2 rounded">
                                                    {tok.token.clone()}
                                                </code>
                                                <button
                                                    type="button"
                                                    class="btn btn-xs btn-ghost mt-2"
                                                    on:click=move |_| set_created_token.set(None)
                                                >
                                                    "Zamknij"
                                                </button>
                                            </div>
                                        </div>
                                    })}

                                    <div class="flex gap-2 mb-4">
                                        <input
                                            type="text"
                                            class="input input-bordered flex-1"
                                            placeholder="Nazwa tokena..."
                                            prop:value=move || new_token_name.get()
                                            on:input=move |ev| set_new_token_name.set(event_target_value(&ev))
                                        />
                                        <button
                                            type="button"
                                            class="btn btn-primary btn-sm"
                                            on:click=move |_| {
                                                let name = new_token_name.get();
                                                if name.trim().is_empty() { return; }
                                                leptos::task::spawn_local(async move {
                                                    match create_token_sf(name, "api".to_string()).await {
                                                        Ok(tok) => {
                                                            set_created_token.set(Some(tok));
                                                            set_new_token_name.set(String::new());
                                                            set_refresh.update(|n| *n += 1);
                                                        }
                                                        Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                                    }
                                                });
                                            }
                                        >
                                            "Utwórz"
                                        </button>
                                    </div>

                                    {if tokens.is_empty() {
                                        view! {
                                            <p class="text-base-content/50 text-sm">"Brak tokenów."</p>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="flex flex-col gap-2">
                                                {tokens.into_iter().map(|tok| {
                                                    let tid = tok.id.clone();
                                                    view! {
                                                        <div class="flex items-center justify-between p-2 bg-base-300 rounded">
                                                            <div>
                                                                <span class="font-medium text-sm">{tok.name.clone()}</span>
                                                                <span class="text-xs text-base-content/50 ml-2">{tok.scope.clone()}</span>
                                                            </div>
                                                            <button
                                                                type="button"
                                                                class="btn btn-ghost btn-xs text-error"
                                                                on:click=move |_| {
                                                                    let id = tid.clone();
                                                                    leptos::task::spawn_local(async move {
                                                                        match revoke_token_sf(id).await {
                                                                            Ok(_) => set_refresh.update(|n| *n += 1),
                                                                            Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                                                        }
                                                                    });
                                                                }
                                                            >
                                                                "Unieważnij"
                                                            </button>
                                                        </div>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        }.into_any()
                                    }}
                                </div>

                                // --- Admin section ---
                                {if is_admin {
                                    view! {
                                        <div class="card bg-base-200 p-4 border border-warning/30">
                                            <h3 class="font-semibold mb-3 text-warning">"Panel admina"</h3>
                                            <div class="flex items-center gap-3">
                                                <span class="text-sm">"Rejestracja nowych użytkowników:"</span>
                                                <input
                                                    type="checkbox"
                                                    class="toggle toggle-warning"
                                                    prop:checked=reg_enabled
                                                    on:change=move |ev| {
                                                        let checked = event_target_checked(&ev);
                                                        leptos::task::spawn_local(async move {
                                                            match set_reg_enabled(checked).await {
                                                                Ok(_) => set_refresh.update(|n| *n += 1),
                                                                Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                                            }
                                                        });
                                                    }
                                                />
                                                <span class="text-sm text-base-content/50">
                                                    {if reg_enabled { "włączona" } else { "wyłączona" }}
                                                </span>
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {}.into_any()
                                }}
                            </div>
                        }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}
