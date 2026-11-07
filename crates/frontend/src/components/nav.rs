use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::hooks::use_location;

use kartoteka_shared::ChangeTopic;

use crate::components::ui::Icon;
use crate::server_fns::auth::{do_logout, get_nav_data};
use crate::server_fns::pending_deletions::get_pending_deletion_count;
use crate::state::invalidation::use_invalidation;

/// Topics whose changes can add or remove rows in the pending-deletions queue.
const TRASH_TOPICS: [ChangeTopic; 4] = [
    ChangeTopic::Items,
    ChangeTopic::Lists,
    ChangeTopic::Containers,
    ChangeTopic::Comments,
];

#[component]
pub fn Nav() -> impl IntoView {
    let location = use_location();
    let pathname = location.pathname;
    let invalidation = use_invalidation();
    let nav = Resource::new(move || pathname.get(), |_| get_nav_data());
    // Refetched on navigation (like the nav data) and whenever a mutation could have
    // changed the queue.
    let pending = Resource::new(
        move || (pathname.get(), invalidation.track(&TRASH_TOPICS)),
        |_| get_pending_deletion_count(),
    );

    view! {
        <nav class="navbar bg-base-100 border-b border-base-300">
            <div class="navbar-start">
                <a href="/" class="btn btn-ghost text-xl">"Kartoteka"</a>
            </div>
            <div class="navbar-end">
                <Suspense>
                    {move || match nav.get().and_then(|r| r.ok()) {
                        Some(name) => view! {
                            <a href="/today" class="btn btn-ghost btn-sm">{move_tr!("nav-today")}</a>
                            <a href="/calendar" class="btn btn-ghost btn-sm">{move_tr!("nav-calendar")}</a>
                            <a href="/tags" class="btn btn-ghost btn-sm" data-testid="nav-tags">{move_tr!("nav-tags")}</a>
                            <a href="/locations" class="btn btn-ghost btn-sm">{move_tr!("locations-title")}</a>
                            <a href="/all" class="btn btn-ghost btn-sm">{move_tr!("nav-all")}</a>
                            <div class="dropdown dropdown-end">
                                <div tabindex="0" role="button" class="btn btn-ghost btn-sm" data-testid="nav-user-menu">
                                    {name}
                                    <Icon name="chevron-down" size=14 />
                                </div>
                                <ul tabindex="0" class="dropdown-content menu bg-base-100 rounded-box z-50 w-52 p-2 shadow-lg border border-base-300">
                                    <li>
                                        <a href="/trash" data-testid="nav-trash">
                                            <Icon name="trash" size=16 />
                                            {move_tr!("nav-trash")}
                                            <Suspense fallback=|| ()>
                                                {move || {
                                                    pending
                                                        .get()
                                                        .and_then(|r| r.ok())
                                                        .filter(|count| *count > 0)
                                                        .map(|count| view! {
                                                            <span
                                                                class="badge badge-error badge-sm"
                                                                data-testid="nav-trash-count"
                                                            >
                                                                {count}
                                                            </span>
                                                        })
                                                }}
                                            </Suspense>
                                        </a>
                                    </li>
                                    <li>
                                        <a href="/settings">
                                            <Icon name="settings" size=16 />
                                            {move_tr!("nav-settings")}
                                        </a>
                                    </li>
                                    <li>
                                        <button
                                            type="button"
                                            on:click=move |_| {
                                                leptos::task::spawn_local(async move {
                                                    let _ = do_logout().await;
                                                });
                                            }
                                        >
                                            <Icon name="log-out" size=16 />
                                            {move_tr!("nav-logout")}
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
