use leptos::prelude::*;
use leptos_fluent::move_tr;

/// Shared loading spinner with i18n text.
#[component]
pub fn LoadingSpinner() -> impl IntoView {
    view! {
        <div class="flex justify-center items-center p-8">
            <span class="loading loading-spinner loading-lg"></span>
            <span class="ml-2">{move_tr!("common-loading")}</span>
        </div>
    }
}
