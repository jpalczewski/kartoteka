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
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-4">"Ustawienia"</h2>
            <div class="card bg-base-200 border border-base-300">
                <div class="card-body">
                    <hanko-profile></hanko-profile>
                </div>
            </div>
        </div>
    }
}
