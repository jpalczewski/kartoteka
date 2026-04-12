use leptos::prelude::*;

#[component]
pub fn Nav() -> impl IntoView {
    view! {
        <nav class="navbar bg-base-100 border-b border-base-300">
            <div class="navbar-start">
                <a href="/" class="btn btn-ghost text-xl">"Kartoteka"</a>
            </div>
        </nav>
    }
}
