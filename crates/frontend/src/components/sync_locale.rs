use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_fluent::I18n;

use crate::api;
use crate::api::client::GlooClient;

#[component]
pub fn SyncLocale() -> impl IntoView {
    let i18n = expect_context::<I18n>();
    let client = use_context::<GlooClient>().expect("GlooClient not provided");

    spawn_local(async move {
        match api::preferences::get_preferences(&client).await {
            Ok(prefs) => {
                if let Some(lang) = i18n
                    .languages
                    .iter()
                    .find(|l| l.id.to_string() == prefs.locale)
                {
                    i18n.language.set(lang);
                }
            }
            Err(api::ApiError::Http { status: 401, .. }) => {
                // Unauthenticated page (login/signup) — do nothing
            }
            Err(_) => {
                // Network error or preference fetch failed — keep navigator/localStorage default
            }
        }
    });

    view! { <></> }
}
