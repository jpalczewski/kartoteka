use leptos::prelude::*;

use crate::api;

#[component]
pub fn Nav() -> impl IntoView {
    let (menu_open, set_menu_open) = signal(false);
    let email = api::get_user_email().unwrap_or_default();
    let logged_in = api::is_logged_in();

    let on_logout = move |_| {
        api::logout();
    };

    view! {
        <nav class="navbar bg-base-200 border-b border-base-300 px-4">
            <div class="navbar-start">
                <a href="/" style="text-decoration: none;">
                    <span class="text-xl font-bold text-primary">"Kartoteka"</span>
                </a>
            </div>

            <div class="navbar-end">
                {if logged_in {
                    let email_display = if email.is_empty() {
                        "Konto".to_string()
                    } else {
                        email.clone()
                    };

                    view! {
                        <a href="/today" class="btn btn-ghost btn-sm">"Dziś"</a>
                        <div class="relative">
                            <button
                                class="btn btn-ghost btn-sm"
                                on:click=move |_| set_menu_open.update(|v| *v = !*v)
                            >
                                {email_display}
                            </button>
                            <ul
                                class="menu bg-base-200 rounded-box border border-base-300 shadow-lg z-50 min-w-40 absolute right-0 top-full mt-1"
                                style:display=move || if menu_open.get() { "block" } else { "none" }
                            >
                                <li><a href="/tags">"Tagi"</a></li>
                                <li><a href="/settings">"Ustawienia"</a></li>
                                <li><button type="button" on:click=on_logout>"Wyloguj"</button></li>
                            </ul>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <a href="/login" class="btn btn-primary btn-sm">"Zaloguj"</a>
                    }.into_any()
                }}
            </div>
        </nav>
    }
}
