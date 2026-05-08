use leptos::prelude::*;
use leptos_fluent::move_tr;

#[component]
pub fn LandingPage() -> impl IntoView {
    view! {
        <div class="flex min-h-[70vh] flex-col items-center justify-center gap-6 text-center px-4">
            <h1 class="text-4xl font-bold">{move_tr!("app-title")}</h1>
            <p class="text-lg text-base-content/70 max-w-sm">{move_tr!("landing-tagline")}</p>
            <div class="flex gap-3">
                <a href="/login" class="btn btn-primary">{move_tr!("nav-login")}</a>
                <a href="/signup" class="btn btn-ghost">{move_tr!("auth-login-create-account")}</a>
            </div>
        </div>
    }
}
