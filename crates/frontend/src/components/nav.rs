use leptos::prelude::*;
use leptos_fluent::move_tr;

use crate::api;
use crate::app::SessionResource;
use crate::state::AdminContext;

#[component]
pub fn Nav() -> impl IntoView {
    let (menu_open, set_menu_open) = signal(false);
    let admin_ctx = use_context::<AdminContext>();

    let session_res = use_context::<SessionResource>().expect("SessionResource missing");

    let on_logout = move |_| {
        api::logout();
    };

    view! {
        <nav class="navbar bg-base-200 border-b border-base-300 px-4">
            <div class="navbar-start">
                <a href="/" style="text-decoration: none;">
                    <span class="text-xl font-bold text-primary">{move_tr!("app-title")}</span>
                </a>
            </div>

            <div class="navbar-end">
                <Suspense fallback=|| view! { <span class="loading loading-spinner loading-sm"></span> }>
                    {move || {
                        session_res.get().map(|s| {
                            match s.as_ref() {
                                Some(session) => {
                                    let email_display = session.user.email.clone();
                                    view! {
                                        <a href="/today" class="btn btn-ghost btn-sm">{move_tr!("nav-today")}</a>
                                        <a href="/calendar" class="btn btn-ghost btn-sm">{move_tr!("nav-calendar")}</a>
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
                                                {move || admin_ctx.map(|ctx| {
                                                    ctx.is_admin.get().then(|| view! {
                                                        <li><a href="/admin">{move_tr!("nav-admin")}</a></li>
                                                    })
                                                })}
                                                <li><a href="/tags">{move_tr!("nav-tags")}</a></li>
                                                <li><a href="/settings">{move_tr!("nav-settings")}</a></li>
                                                <li><button type="button" on:click=on_logout>{move_tr!("nav-logout")}</button></li>
                                            </ul>
                                        </div>
                                    }.into_any()
                                }
                                None => view! {
                                    <a href="/login" class="btn btn-primary btn-sm">{move_tr!("nav-login")}</a>
                                }.into_any(),
                            }
                        })
                    }}
                </Suspense>
            </div>
        </nav>
    }
}
