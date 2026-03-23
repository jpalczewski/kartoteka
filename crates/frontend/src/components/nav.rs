use leptos::prelude::*;

#[component]
pub fn Nav() -> impl IntoView {
    view! {
        <nav>
            <h1>
                <a href="/" style="color: inherit; text-decoration: none;">"Kartoteka"</a>
            </h1>
        </nav>
    }
}
