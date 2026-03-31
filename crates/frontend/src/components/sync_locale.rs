use leptos::prelude::*;
use leptos_fluent::I18n;

use crate::api;
use crate::api::client::GlooClient;
use crate::app::SessionResource;

#[component]
pub fn SyncLocale() -> impl IntoView {
    let i18n = expect_context::<I18n>();
    let client = use_context::<GlooClient>().expect("GlooClient not provided");
    let session_res = use_context::<SessionResource>().expect("SessionResource missing");

    let prefs_res = {
        let client = client.clone();
        LocalResource::new(move || {
            let client = client.clone();
            let session = session_res.get();
            async move {
                match session {
                    Some(Some(_)) => api::preferences::get_preferences(&client).await.ok(),
                    _ => None,
                }
            }
        })
    };

    Effect::new(move |_| {
        if let Some(Some(prefs)) = prefs_res.get() {
            if let Some(lang) = i18n.languages.iter().find(|l| l.id == prefs.locale) {
                i18n.language.set(lang);
            }
        }
    });

    view! { <></> }
}
