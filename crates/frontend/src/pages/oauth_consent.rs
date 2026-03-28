use leptos::prelude::*;

use crate::api;

#[component]
pub fn OAuthConsentPage() -> impl IntoView {
    let query_string = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default();
    let query_string = query_string
        .strip_prefix('?')
        .unwrap_or(&query_string)
        .to_string();

    let params = web_sys::UrlSearchParams::new_with_str(&query_string).ok();

    let gateway_approve_url = format!("{}/oauth/authorize?{}", api::auth_base(), query_string);

    let deny_url = params
        .as_ref()
        .and_then(|p| p.get("redirect_uri"))
        .map(|uri| {
            let state = params
                .as_ref()
                .and_then(|p| p.get("state"))
                .unwrap_or_default();
            format!("{}?error=access_denied&state={}", uri, state)
        })
        .unwrap_or_else(|| "/".to_string());

    view! {
        <div class="container mx-auto max-w-md p-4 mt-16">
            <div class="card bg-base-200 border border-base-300">
                <div class="card-body">
                    <h2 class="card-title text-xl">"Autoryzacja Claude"</h2>
                    <p class="mt-2">
                        "Claude prosi o dostęp do Twoich list w Kartotece."
                    </p>
                    <div class="card-actions justify-end mt-4 gap-2">
                        <a href={deny_url} class="btn btn-ghost">
                            "Odmów"
                        </a>
                        <a href={gateway_approve_url} class="btn btn-primary">
                            "Zezwól"
                        </a>
                    </div>
                </div>
            </div>
        </div>
    }
}
