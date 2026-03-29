use leptos::prelude::*;
use leptos_fluent::move_tr;

#[component]
pub fn SignupPage() -> impl IntoView {
    let name = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(false);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        loading.set(true);
        error.set(None);

        leptos::task::spawn_local(async move {
            let body = serde_json::json!({
                "name": name.get_untracked(),
                "email": email.get_untracked(),
                "password": password.get_untracked(),
            });
            let url = format!("{}/auth/api/sign-up/email", crate::api::auth_base());
            let json = match serde_json::to_string(&body) {
                Ok(s) => s,
                Err(e) => {
                    loading.set(false);
                    error.set(Some(format!("Błąd: {e}")));
                    return;
                }
            };
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
            let result = request.send().await;

            loading.set(false);
            match result {
                Ok(resp) if resp.ok() => {
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/");
                    }
                }
                Ok(resp) => {
                    error.set(Some(format!("Błąd rejestracji ({})", resp.status())));
                }
                Err(e) => {
                    error.set(Some(format!("Błąd sieci: {e}")));
                }
            }
        });
    };

    view! {
        <div class="flex flex-col items-center justify-center min-h-[60vh] p-4">
            <div class="card bg-base-200 border border-base-300 w-full max-w-sm">
                <div class="card-body items-center">
                    <h2 class="card-title text-2xl mb-4">{move_tr!("auth-signup-title")}</h2>

                    {move || error.get().map(|e| view! {
                        <div class="alert alert-error mb-4">
                            <span>{e}</span>
                        </div>
                    })}

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
                </div>
            </div>
        </div>
    }
}
