use leptos::prelude::*;

#[component]
pub fn LoadingSpinner() -> impl IntoView {
    view! {
        <div class="flex justify-center py-8">
            <span class="loading loading-spinner loading-md text-primary"></span>
        </div>
    }
}
