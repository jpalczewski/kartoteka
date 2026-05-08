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
    let check = Resource::new(
        || (),
        move |_| {
            let path = full_path();
            require_auth(path)
        },
    );
    view! {
        <Suspense fallback=|| view! { <LoadingSpinner/> }>
            {move || match check.get() {
                None => ().into_any(),
                Some(Ok(_)) => children().into_any(),
                Some(Err(_)) => {
                    #[cfg(target_arch = "wasm32")]
                    {
                        let path = full_path();
                        let encoded = urlencoding::encode(&path);
                        if let Some(win) = web_sys::window() {
                            let _ = win.location().set_href(&format!("/login?return_to={}", encoded));
                        }
                    }
                    view! { <LoadingSpinner/> }.into_any()
                }
            }}
        </Suspense>
    }
}
