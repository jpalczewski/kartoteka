use leptos::prelude::*;

use crate::server_fns::auth::do_register;
use crate::server_fns::settings::is_reg_enabled;

#[component]
pub fn SignupPage() -> impl IntoView {
    let reg_enabled_res = Resource::new(|| (), |_| is_reg_enabled());

    let (name, set_name) = signal(String::new());
    let (email, set_email) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (loading, set_loading) = signal(false);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let e = email.get();
        let p = password.get();
        let n = name.get();
        if e.trim().is_empty() || p.is_empty() {
            set_error.set(Some("Podaj email i hasło.".to_string()));
            return;
        }
        set_loading.set(true);
        set_error.set(None);
        let name_opt = if n.trim().is_empty() { None } else { Some(n) };
        leptos::task::spawn_local(async move {
            match do_register(e, p, name_opt).await {
                Ok(_) => {
                    // redirect to /login handled by server
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
                <h2 class="text-2xl font-bold mb-6 text-center">"Rejestracja"</h2>

                <Suspense fallback=|| view! { <span class="loading loading-spinner"/> }>
                    {move || reg_enabled_res.get().map(|result| {
                        let enabled = result.unwrap_or(false);
                        if !enabled {
                            return view! {
                                <div class="alert alert-warning">
                                    "Rejestracja jest obecnie wyłączona. Skontaktuj się z administratorem."
                                </div>
                            }.into_any();
                        }
                        view! {
                            <form on:submit=on_submit class="flex flex-col gap-4">
                                <div class="form-control">
                                    <label class="label">
                                        <span class="label-text">"Imię (opcjonalnie)"</span>
                                    </label>
                                    <input
                                        type="text"
                                        class="input input-bordered"
                                        placeholder="Twoje imię"
                                        prop:value=move || name.get()
                                        on:input=move |ev| set_name.set(event_target_value(&ev))
                                    />
                                </div>

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
                                        view! { "Zarejestruj" }.into_any()
                                    }}
                                </button>
                            </form>
                        }.into_any()
                    })}
                </Suspense>

                <div class="text-center mt-4 text-sm text-base-content/60">
                    "Masz już konto? "
                    <a href="/login" class="link link-primary">"Zaloguj się"</a>
                </div>
            </div>
        </div>
    }
}
