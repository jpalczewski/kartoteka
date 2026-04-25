use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::server_fns::auth::{LoginOutcome, do_login, do_verify_2fa};

#[derive(Clone, PartialEq)]
enum LoginStage {
    Credentials,
    TwoFa,
}

#[component]
pub fn LoginPage() -> impl IntoView {
    let query = use_query_map();
    let return_to = move || query.with(|q| q.get("return_to").map(|s| s.to_string()));

    let (email, set_email) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (totp_code, set_totp_code) = signal(String::new());
    let (stage, set_stage) = signal(LoginStage::Credentials);
    let (error, set_error) = signal(Option::<String>::None);
    let (loading, set_loading) = signal(false);

    let on_submit_credentials = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let e = email.get();
        let p = password.get();
        let rt = return_to();
        if e.trim().is_empty() || p.is_empty() {
            set_error.set(Some("Podaj email i hasło.".to_string()));
            return;
        }
        set_loading.set(true);
        set_error.set(None);
        leptos::task::spawn_local(async move {
            match do_login(e, p, rt).await {
                Ok(LoginOutcome::Ok) => {}
                Ok(LoginOutcome::TwoFaRequired) => {
                    set_loading.set(false);
                    set_stage.set(LoginStage::TwoFa);
                }
                Err(e) => {
                    set_error.set(Some(e.to_string()));
                    set_loading.set(false);
                }
            }
        });
    };

    let on_submit_totp = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let code = totp_code.get();
        if code.trim().is_empty() {
            set_error.set(Some("Wprowadź kod weryfikacyjny.".to_string()));
            return;
        }
        set_loading.set(true);
        set_error.set(None);
        leptos::task::spawn_local(async move {
            match do_verify_2fa(code).await {
                Ok(_) => {}
                Err(e) => {
                    let msg = match e.to_string().as_str() {
                        s if s.contains("invalid_code") => "Nieprawidłowy kod.".to_string(),
                        s if s.contains("too_many_attempts") => {
                            "Za dużo prób. Zaloguj się ponownie hasłem.".to_string()
                        }
                        other => other.to_string(),
                    };
                    set_error.set(Some(msg));
                    set_loading.set(false);
                }
            }
        });
    };

    let on_back = move |_| {
        set_stage.set(LoginStage::Credentials);
        set_totp_code.set(String::new());
        set_error.set(None);
    };

    view! {
        <div class="flex min-h-[70vh] items-center justify-center">
            <div class="card bg-base-200 w-full max-w-sm p-6">
                {move || match stage.get() {
                    LoginStage::Credentials => view! {
                        <h2 class="text-2xl font-bold mb-6 text-center">"Logowanie"</h2>
                        <form on:submit=on_submit_credentials class="flex flex-col gap-4">
                            <div class="form-control">
                                <label class="label">
                                    <span class="label-text">"Email"</span>
                                </label>
                                <input
                                    type="email"
                                    class="input input-bordered"
                                    placeholder="adres@email.pl"
                                    prop:value=move || email.get()
                                    on:input=move |ev| set_email.set(event_target_value(&ev))
                                    required=true
                                />
                            </div>
                            <div class="form-control">
                                <label class="label">
                                    <span class="label-text">"Hasło"</span>
                                </label>
                                <input
                                    type="password"
                                    class="input input-bordered"
                                    placeholder="••••••••"
                                    prop:value=move || password.get()
                                    on:input=move |ev| set_password.set(event_target_value(&ev))
                                    required=true
                                />
                            </div>
                            {move || error.get().map(|e| view! {
                                <div class="alert alert-error text-sm">{e}</div>
                            })}
                            <button
                                type="submit"
                                class="btn btn-primary w-full"
                                disabled=move || loading.get()
                            >
                                {move || if loading.get() {
                                    view! { <span class="loading loading-spinner loading-sm"/> }.into_any()
                                } else {
                                    view! { "Zaloguj" }.into_any()
                                }}
                            </button>
                        </form>
                        <div class="text-center mt-4 text-sm text-base-content/60">
                            "Nie masz konta? "
                            <a href="/signup" class="link link-primary">"Zarejestruj się"</a>
                        </div>
                    }.into_any(),
                    LoginStage::TwoFa => view! {
                        <h2 class="text-2xl font-bold mb-2 text-center">"Weryfikacja dwuetapowa"</h2>
                        <p class="text-sm text-base-content/60 text-center mb-4">
                            "Wprowadź 6-cyfrowy kod z aplikacji uwierzytelniającej."
                        </p>
                        <form on:submit=on_submit_totp class="flex flex-col gap-4">
                            <div class="form-control">
                                <label class="label">
                                    <span class="label-text">"Kod weryfikacyjny"</span>
                                </label>
                                <input
                                    type="text"
                                    inputmode="numeric"
                                    autocomplete="one-time-code"
                                    class="input input-bordered text-center tracking-widest text-lg"
                                    placeholder="000000"
                                    maxlength="6"
                                    prop:value=move || totp_code.get()
                                    on:input=move |ev| set_totp_code.set(event_target_value(&ev))
                                    required=true
                                />
                            </div>
                            {move || error.get().map(|e| view! {
                                <div class="alert alert-error text-sm">{e}</div>
                            })}
                            <button
                                type="submit"
                                class="btn btn-primary w-full"
                                disabled=move || loading.get()
                            >
                                {move || if loading.get() {
                                    view! { <span class="loading loading-spinner loading-sm"/> }.into_any()
                                } else {
                                    view! { "Weryfikuj" }.into_any()
                                }}
                            </button>
                            <button type="button" class="btn btn-ghost btn-sm" on:click=on_back>
                                "← Wróć"
                            </button>
                        </form>
                    }.into_any(),
                }}
            </div>
        </div>
    }
}
