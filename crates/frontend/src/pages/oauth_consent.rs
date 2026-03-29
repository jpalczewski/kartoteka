use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_fluent::move_tr;

use crate::api;

#[component]
pub fn OAuthConsentPage() -> impl IntoView {
    let query_string = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default();
    let query_string = query_string
        .strip_prefix('?')
        .unwrap_or(&query_string)
        .to_string();

    let params = web_sys::UrlSearchParams::new_with_str(&query_string).ok();
    let redirect_uri = params
        .as_ref()
        .and_then(|p| p.get("redirect_uri"))
        .unwrap_or_default();
    let state = params
        .as_ref()
        .and_then(|p| p.get("state"))
        .unwrap_or_default();
    let deny_url = format!("{}?error=access_denied&state={}", redirect_uri, state);

    let session = LocalResource::new(api::get_session);
    let login_error = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(false);
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());

    let qs_for_login = query_string.clone();
    let on_login = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        loading.set(true);
        login_error.set(None);
        let qs = qs_for_login.clone();
        spawn_local(async move {
            let body = serde_json::json!({
                "email": email.get_untracked(),
                "password": password.get_untracked(),
            });
            let url = format!("{}/auth/api/sign-in/email", api::auth_base());
            let json = serde_json::to_string(&body).unwrap_or_default();
            let result = gloo_net::http::Request::post(&url)
                .header("Content-Type", "application/json")
                .credentials(web_sys::RequestCredentials::Include)
                .body(json)
                .unwrap()
                .send()
                .await;
            loading.set(false);
            match result {
                Ok(resp) if resp.ok() => {
                    // Logged in — now get consent token and redirect
                    approve_flow(&qs).await;
                }
                Ok(resp) => {
                    let msg = resp
                        .json::<serde_json::Value>()
                        .await
                        .ok()
                        .and_then(|v| v.get("message").and_then(|m| m.as_str().map(String::from)))
                        .unwrap_or_else(|| format!("Błąd logowania ({})", resp.status()));
                    login_error.set(Some(msg));
                }
                Err(e) => {
                    login_error.set(Some(format!("Błąd sieci: {e}")));
                }
            }
        });
    };

    let qs_for_approve = query_string.clone();
    let on_approve = move |_: web_sys::MouseEvent| {
        let qs = qs_for_approve.clone();
        spawn_local(async move {
            approve_flow(&qs).await;
        });
    };

    view! {
        <div class="container mx-auto max-w-md p-4 mt-16">
            <Suspense fallback=move || view! {
                <div class="flex justify-center py-8">
                    <span class="loading loading-spinner loading-lg"></span>
                </div>
            }>
                {move || {
                    let deny_url = deny_url.clone();
                    session.get().map(|result| {
                        let session_info = result.clone();
                        match session_info {
                            Some(info) => {
                                // Logged in — show consent
                                view! {
                                    <div class="card bg-base-200 border border-base-300">
                                        <div class="card-body">
                                            <h2 class="card-title text-xl">{move_tr!("auth-consent-title")}</h2>
                                            <p class="text-base-content/70">
                                                {move_tr!("auth-consent-logged-in-as")}
                                                " "
                                                <strong>{info.user.email.clone()}</strong>
                                            </p>
                                            <p class="mt-2">
                                                {move_tr!("auth-consent-request")}
                                            </p>
                                            <div class="card-actions justify-end mt-4 gap-2">
                                                <a href={deny_url} class="btn btn-ghost">{move_tr!("auth-consent-deny")}</a>
                                                <button class="btn btn-primary" on:click=on_approve.clone()>
                                                    {move_tr!("auth-consent-allow")}
                                                </button>
                                            </div>
                                        </div>
                                    </div>
                                }.into_any()
                            }
                            None => {
                                // Not logged in — show login form
                                view! {
                                    <div class="card bg-base-200 border border-base-300">
                                        <div class="card-body">
                                            <h2 class="card-title text-xl">{move_tr!("auth-consent-title")}</h2>
                                            <p class="text-base-content/70 mb-2">
                                                {move_tr!("auth-consent-login-prompt")}
                                            </p>

                                            {move || login_error.get().map(|e| view! {
                                                <div class="alert alert-error mb-2">
                                                    <span>{e}</span>
                                                </div>
                                            })}

                                            <form on:submit=on_login.clone() class="space-y-3">
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
                                                <button
                                                    type="submit"
                                                    class="btn btn-primary w-full"
                                                    disabled=move || loading.get()
                                                >
                                                    {move || if loading.get() {
                                                        move_tr!("auth-login-loading").get()
                                                    } else {
                                                        move_tr!("auth-consent-login-and-authorize").get()
                                                    }}
                                                </button>
                                            </form>
                                        </div>
                                    </div>
                                }.into_any()
                            }
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}

async fn approve_flow(query_string: &str) {
    let url = format!("{}/oauth/consent-token", api::auth_base());
    let result = gloo_net::http::Request::post(&url)
        .credentials(web_sys::RequestCredentials::Include)
        .send()
        .await;

    match result {
        Ok(resp) if resp.ok() => {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if let Some(token) = data.get("consent_token").and_then(|t| t.as_str()) {
                    let redirect = format!(
                        "{}/oauth/authorize?consent_token={}&{}",
                        api::auth_base(),
                        token,
                        query_string
                    );
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href(&redirect);
                    }
                }
            }
        }
        _ => {
            if let Some(w) = web_sys::window() {
                let _ = w.location().set_href("/login");
            }
        }
    }
}
