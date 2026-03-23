use leptos::prelude::*;

use crate::api;

#[component]
pub fn SettingsPage() -> impl IntoView {
    if !api::is_logged_in() {
        if let Some(w) = web_sys::window() {
            let _ = w.location().set_href("/login");
        }
    }

    view! {
        <h2 style="margin: 1rem 0;">"Ustawienia"</h2>
        <hanko-profile></hanko-profile>
    }
}
