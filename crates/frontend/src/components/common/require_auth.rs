use leptos::prelude::*;
use leptos_router::hooks::use_location;

use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::auth::require_auth;

/// Wraps protected routes. During SSR: if not authenticated, `require_auth` sets
/// the Location header and redirects before HTML is sent. After hydration: the
/// resource re-runs; unauthenticated users see a spinner until the redirect fires.
#[component]
pub fn RequireAuth(children: ChildrenFn) -> impl IntoView {
    let location = use_location();
    let full_path = move || {
        let p = location.pathname.get();
        let s = location.search.get();
        if s.is_empty() {
            p
        } else {
            format!("{}{}", p, s)
        }
    };
    let check = Resource::new(full_path, require_auth);
    view! {
        <Suspense fallback=|| view! { <LoadingSpinner/> }>
            {move || check.get().and_then(|r| r.ok()).map(|_| children())}
        </Suspense>
    }
}
