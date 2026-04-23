use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

use crate::server_fns::auth::do_login;

#[component]
pub fn LoginPage() -> impl IntoView {
    let query = use_query_map();
    let return_to = move || query.with(|q| q.get("return_to").map(|s| s.to_string()));

    let (email, set_email) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (loading, set_loading) = signal(false);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
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
                Ok(_) => {
                    // redirect handled by server via leptos_axum::redirect("/")
                }
                Err(e) => {
                    set_error.set(Some(e.to_string()));
                    set_loading.set(false);
                }
            }
        });
    };

    view! {
        <div class="flex min-h-[70vh] items-center justify-center">
            <div class="card bg-base-200 w-full max-w-sm p-6">
                <h2 class="text-2xl font-bold mb-6 text-center">"Logowanie"</h2>

                <form on:submit=on_submit class="flex flex-col gap-4">
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
            </div>
        </div>
    }
}
