use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::components::comments::CommentSection;

#[component]
pub fn ContainerPage() -> impl IntoView {
    let params = use_params_map();
    let container_id = Signal::derive(move || params.read().get("id").unwrap_or_default());

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-4">"Kontener"</h2>

            <CommentSection
                entity_type="container"
                entity_id=container_id
            />
        </div>
    }
}
