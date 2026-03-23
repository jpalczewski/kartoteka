use leptos::prelude::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    view! {
        <div style="display: flex; flex-direction: column; align-items: center; padding: 2rem;">
            <h2 style="margin-bottom: 1.5rem;">"Zaloguj się"</h2>
            <hanko-auth id="hankoAuth"></hanko-auth>
        </div>
    }
}
