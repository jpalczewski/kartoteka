use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_fluent::I18n;

use crate::api::preferences::get_preferences;

#[component]
pub fn SyncLocale() -> impl IntoView {
    let i18n = expect_context::<I18n>();

    spawn_local(async move {
        match get_preferences().await {
            Ok(prefs) => {
                if let Some(lang) = i18n
                    .languages
                    .iter()
                    .find(|l| l.id.to_string() == prefs.locale)
                {
                    i18n.language.set(lang);
                }
            }
            Err(ref e) if e == "unauthorized" => {
                // Unauthenticated page (login/signup) — do nothing
            }
            Err(_) => {
                // Network error or preference fetch failed — keep navigator/localStorage default
            }
        }
    });

    view! { <></> }
}
