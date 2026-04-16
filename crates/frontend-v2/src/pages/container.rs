use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::components::comments::CommentSection;
use crate::components::common::loading::LoadingSpinner;
use crate::components::lists::{container_card::ContainerCard, list_card::ListCard};
use crate::server_fns::containers::get_container_data;

fn container_status_icon(status: Option<&str>) -> &'static str {
    match status {
        None => "📁",
        Some("active") => "🚀",
        Some("done") => "✅",
        Some("paused") => "⏸️",
        _ => "📁",
    }
}

#[component]
pub fn ContainerPage() -> impl IntoView {
    let params = use_params_map();
    let container_id = Signal::derive(move || {
        params.read().get("id").unwrap_or_default()
    });

    let data_res = Resource::new(
        move || container_id.get(),
        |cid| get_container_data(cid),
    );

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || data_res.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="text-error">"Błąd: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(data) => {
                        let icon = container_status_icon(data.container.status.as_deref());
                        let name = data.container.name.clone();
                        let desc = data.container.description.clone();
                        let lists = data.lists.clone();
                        let children = data.children.clone();

                        view! {
                            <div class="flex flex-col gap-6">
                                // Header
                                <div class="flex items-center gap-3">
                                    <span class="text-3xl">{icon}</span>
                                    <div>
                                        <h2 class="text-2xl font-bold">{name}</h2>
                                        {desc.map(|d| view! {
                                            <p class="text-base-content/60 text-sm mt-1">{d}</p>
                                        })}
                                    </div>
                                </div>

                                // Child containers
                                {if !children.is_empty() {
                                    view! {
                                        <div>
                                            <h3 class="text-sm font-semibold text-base-content/60 mb-2 uppercase tracking-wide">
                                                "Subkontenerów (" {children.len()} ")"
                                            </h3>
                                            <div class="flex flex-col gap-2">
                                                {children.into_iter().map(|child| view! {
                                                    <ContainerCard container=child />
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {}.into_any()
                                }}

                                // Lists in this container
                                {if lists.is_empty() {
                                    view! {
                                        <div class="text-center text-base-content/50 py-4">
                                            "Brak list w tym kontenerze."
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div>
                                            <h3 class="text-sm font-semibold text-base-content/60 mb-2 uppercase tracking-wide">
                                                "Listy (" {lists.len()} ")"
                                            </h3>
                                            <div class="flex flex-col gap-2">
                                                {lists.into_iter().map(|list| view! {
                                                    <ListCard list=list />
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        </div>
                                    }.into_any()
                                }}

                                // Comments
                                <div>
                                    <h3 class="text-sm font-semibold text-base-content/60 mb-2 uppercase tracking-wide">
                                        "Komentarze"
                                    </h3>
                                    <CommentSection
                                        entity_type="container"
                                        entity_id=container_id
                                    />
                                </div>
                            </div>
                        }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}
