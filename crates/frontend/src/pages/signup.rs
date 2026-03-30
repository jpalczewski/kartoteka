use leptos::prelude::*;
use leptos_fluent::move_tr;

#[cfg(target_arch = "wasm32")]
use crate::api::client::GlooClient;

#[component]
pub fn SignupPage() -> impl IntoView {
    let name = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let invite_code = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(false);

    // Fetch registration mode on mount
    #[cfg(target_arch = "wasm32")]
    let reg_mode = {
        let client = GlooClient;
        LocalResource::new(move || {
            let client = client.clone();
            async move {
                crate::api::admin::get_registration_mode(&client)
                    .await
                    .map(|r| r.mode)
                    .unwrap_or_else(|_| "open".to_string())
            }
        })
    };

    #[cfg(not(target_arch = "wasm32"))]
    let reg_mode = LocalResource::new(|| async { "open".to_string() });

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        loading.set(true);
        error.set(None);

        // Read cached mode synchronously before spawning to avoid an extra network round-trip.
        // The Gateway always validates independently, so stale-by-submit-time is safe.
        #[allow(unused_variables)]
        let current_mode = reg_mode
            .get_untracked()
            .unwrap_or_else(|| "open".to_string());

        leptos::task::spawn_local(async move {
            let current_email = email.get_untracked();
            let code = invite_code.get_untracked();

            // Validate invite code inline if mode is invite
            // (Gateway will also validate, but this gives instant feedback)
            #[cfg(target_arch = "wasm32")]
            {
                let client = GlooClient;

                if current_mode == "closed" {
                    loading.set(false);
                    error.set(Some(move_tr!("registration-closed-message").get()));
                    return;
                }

                if current_mode == "invite" && !code.is_empty() {
                    match crate::api::admin::validate_invite(&client, &code, &current_email).await {
                        Ok(res) if !res.valid => {
                            loading.set(false);
                            let key = match res.error.as_deref() {
                                Some("code_in_use") => move_tr!("invite-code-in-use").get(),
                                _ => move_tr!("invite-code-invalid").get(),
                            };
                            error.set(Some(key));
                            return;
                        }
                        Err(_) => {
                            loading.set(false);
                            error.set(Some(
                                move_tr!("error-network", { "detail" => "connection error" }).get(),
                            ));
                            return;
                        }
                        _ => {}
                    }
                }
            }

            let mut body = serde_json::json!({
                "name": name.get_untracked(),
                "email": current_email,
                "password": password.get_untracked(),
            });
            if !code.is_empty() {
                body["inviteCode"] = serde_json::Value::String(code.clone());
            }

            let url = format!("{}/auth/api/sign-up/email", crate::api::auth_base());
            let json = match serde_json::to_string(&body) {
                Ok(s) => s,
                Err(e) => {
                    loading.set(false);
                    error.set(Some(format!("Błąd: {e}")));
                    return;
                }
            };

            #[cfg(target_arch = "wasm32")]
            {
                let request = match gloo_net::http::Request::post(&url)
                    .header("Content-Type", "application/json")
                    .credentials(web_sys::RequestCredentials::Include)
                    .body(json)
                {
                    Ok(r) => r,
                    Err(e) => {
                        loading.set(false);
                        error.set(Some(format!("Błąd: {e}")));
                        return;
                    }
                };

                loading.set(false);
                match request.send().await {
                    Ok(resp) if resp.ok() => {
                        // Finalize invite code via /api/me after successful signup
                        if !code.is_empty() {
                            let client = GlooClient;
                            let _ = crate::api::admin::get_me(&client, Some(&code)).await;
                        } else {
                            let client = GlooClient;
                            let _ = crate::api::admin::get_me(&client, None).await;
                        }
                        if let Some(window) = web_sys::window() {
                            let _ = window.location().set_href("/");
                        }
                    }
                    Ok(resp) => {
                        let status = resp.status();
                        let msg = resp
                            .text()
                            .await
                            .ok()
                            .and_then(|t| {
                                serde_json::from_str::<serde_json::Value>(&t)
                                    .ok()
                                    .and_then(|v| v.get("error")?.as_str().map(String::from))
                            })
                            .unwrap_or_else(|| format!("Błąd rejestracji ({status})"));
                        error.set(Some(msg));
                    }
                    Err(e) => {
                        error.set(Some(format!("Błąd sieci: {e}")));
                    }
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = json;
                let _ = url;
                loading.set(false);
            }
        });
    };

    view! {
        <div class="flex flex-col items-center justify-center min-h-[60vh] p-4">
            <div class="card bg-base-200 border border-base-300 w-full max-w-sm">
                <div class="card-body items-center">
                    <h2 class="card-title text-2xl mb-4">{move_tr!("auth-signup-title")}</h2>

                    // Show "registration closed" message if mode is closed
                    <Suspense>
                        {move || reg_mode.get().map(|mode| {
                            if mode.as_str() == "closed" {
                                view! {
                                    <div class="alert alert-warning mb-4">
                                        <span>{move_tr!("registration-closed-message")}</span>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }
                        })}
                    </Suspense>

                    {move || error.get().map(|e| view! {
                        <div class="alert alert-error mb-4">
                            <span>{e}</span>
                        </div>
                    })}

                    // Hide form when closed
                    <Show when=move || {
                        reg_mode.get().map(|m| m.as_str() != "closed").unwrap_or(true)
                    }>
                        <form on:submit=on_submit class="w-full space-y-4">
                            <label class="input input-bordered flex items-center gap-2 w-full">
                                <input
                                    type="text"
                                    placeholder=move_tr!("auth-name-placeholder")
                                    class="grow"
                                    required=true
                                    on:input=move |ev| name.set(event_target_value(&ev))
                                />
                            </label>
                            <label class="input input-bordered flex items-center gap-2 w-full">
                                <input
                                    type="email"
                                    placeholder=move_tr!("auth-email-placeholder")
                                    class="grow"
                                    required=true
                                    on:input=move |ev| email.set(event_target_value(&ev))
                                />
                            </label>
                            <label class="input input-bordered flex items-center gap-2 w-full">
                                <input
                                    type="password"
                                    placeholder=move_tr!("auth-password-placeholder")
                                    class="grow"
                                    required=true
                                    on:input=move |ev| password.set(event_target_value(&ev))
                                />
                            </label>

                            // Show invite code field when mode is "invite"
                            <Show when=move || {
                                reg_mode.get().map(|m| m.as_str() == "invite").unwrap_or(false)
                            }>
                                <label class="input input-bordered flex items-center gap-2 w-full">
                                    <input
                                        type="text"
                                        placeholder=move_tr!("invite-code")
                                        class="grow"
                                        required=true
                                        on:input=move |ev| invite_code.set(event_target_value(&ev))
                                    />
                                </label>
                            </Show>

                            <button
                                type="submit"
                                class="btn btn-primary w-full"
                                disabled=move || loading.get()
                            >
                                {move || if loading.get() {
                                    move_tr!("auth-signup-loading").get()
                                } else {
                                    move_tr!("auth-signup-button").get()
                                }}
                            </button>
                        </form>

                        <div class="divider">{move_tr!("common-or")}</div>
                        <a href="/login" class="link link-primary">{move_tr!("auth-signup-have-account")}</a>
                    </Show>
                </div>
            </div>
        </div>
    }
}
