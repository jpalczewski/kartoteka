use leptos::prelude::*;

#[component]
pub fn SettingsPage() -> impl IntoView {
    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-4">"Ustawienia"</h2>
            <div class="card bg-base-200 border border-base-300">
                <div class="card-body">
                    <p class="text-base-content/60">"Ustawienia konta"</p>
                </div>
            </div>
        </div>
    }
}
