use leptos::prelude::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center min-h-[60vh] p-4">
            <div class="card bg-base-200 border border-base-300 w-full max-w-sm">
                <div class="card-body items-center">
                    <h2 class="card-title text-2xl mb-4">"Zaloguj się"</h2>
                    <hanko-auth id="hankoAuth"></hanko-auth>
                </div>
            </div>
        </div>
    }
}
