use leptos::prelude::*;
use leptos_fluent::move_tr;

use crate::server_fns::auth::{do_logout, get_nav_data};

#[component]
pub fn Nav() -> impl IntoView {
    let nav = Resource::new(|| (), |_| get_nav_data());

    view! {
        <nav class="navbar bg-base-100 border-b border-base-300">
            <div class="navbar-start">
                <a href="/" class="btn btn-ghost text-xl">"Kartoteka"</a>
            </div>
            <div class="navbar-end">
                <Suspense>
                    {move || match nav.get().and_then(|r| r.ok()) {
                        Some(name) => view! {
                            <a href="/today" class="btn btn-ghost btn-sm">"Dziś"</a>
                            <a href="/calendar" class="btn btn-ghost btn-sm">"Terminarz"</a>
                            <a href="/tags" class="btn btn-ghost btn-sm" data-testid="nav-tags">"Tagi"</a>
                            <a href="/all" class="btn btn-ghost btn-sm">"Wszystkie"</a>
                            <div class="dropdown dropdown-end">
                                <div tabindex="0" role="button" class="btn btn-ghost btn-sm">
                                    {name} " ▾"
                                </div>
                                <ul tabindex="0" class="dropdown-content menu bg-base-100 rounded-box z-50 w-52 p-2 shadow-lg border border-base-300">
                                    <li><a href="/settings">"⚙ Ustawienia"</a></li>
                                    <li>
                                        <button
                                            type="button"
                                            on:click=move |_| {
                                                leptos::task::spawn_local(async move {
                                                    let _ = do_logout().await;
                                                });
                                            }
                                        >
                                            "⏻ Wyloguj"
                                        </button>
                                    </li>
                                </ul>
                            </div>
                        }.into_any(),
                        None => view! {
                            <a href="/login" class="btn btn-primary btn-sm">
                                {move_tr!("nav-login")}
                            </a>
                        }.into_any(),
                    }}
                </Suspense>
            </div>
        </nav>
    }
}
