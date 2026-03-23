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
        <nav>
            <a href="/" style="color: inherit; text-decoration: none;">
                <h1>"Kartoteka"</h1>
            </a>

            {if logged_in {
                let email_display = if email.is_empty() {
                    "Konto".to_string()
                } else {
                    email.clone()
                };

                view! {
                    <div class="user-menu">
                        <button
                            class="user-menu-trigger"
                            on:click=move |_| set_menu_open.update(|v| *v = !*v)
                        >
                            {email_display}
                        </button>
                        <div
                            class="user-menu-dropdown"
                            style:display=move || if menu_open.get() { "block" } else { "none" }
                        >
                            <a href="/settings" class="user-menu-item">"Ustawienia"</a>
                            <button class="user-menu-item" on:click=on_logout>"Wyloguj"</button>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! {
                    <a href="/login" class="btn" style="font-size: 0.8rem; padding: 0.3rem 0.75rem;">"Zaloguj"</a>
                }.into_any()
            }}
        </nav>
    }
}
