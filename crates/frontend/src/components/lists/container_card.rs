use kartoteka_shared::types::Container;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::components::common::dnd::{DragHandleButton, DragShell, DragSurface};
use crate::state::dnd::{DndState, DropTarget, EntityKind};

pub fn container_icon(status: Option<&str>) -> &'static str {
    match status {
        None => "📁",
        Some("active") => "🚀",
        Some("done") => "✅",
        Some("paused") => "⏸️",
        _ => "📁",
    }
}

#[component]
pub fn ContainerCard(
    container: Container,
    #[prop(optional)] on_delete: Option<Callback<String>>,
    #[prop(optional)] on_pin: Option<Callback<String>>,
    /// Enable drag handle + nest drop zone when provided.
    #[prop(optional)]
    dnd_state: Option<RwSignal<DndState>>,
    #[prop(optional)] on_nest_drop: Option<Callback<DropTarget>>,
) -> impl IntoView {
    let href = format!("/containers/{}", container.id);
    let icon = container_icon(container.status.as_deref());
    let is_pinned = container.pinned;
    let navigate = use_navigate();
    let href_nav = href.clone();

    let cid = container.id.clone();
    let cid_del = container.id.clone();
    let cid_pin = container.id.clone();
    let name = container.name.clone();

    let body = view! {
        <div
            class="card bg-base-200 border border-base-300 cursor-pointer card-neon relative flex-1"
            data-testid="container-card"
            on:click=move |_| { navigate(&href_nav, Default::default()); }
        >
            <div class="card-body p-4">
                <div class="flex items-center justify-between">
                    <div class="flex items-center gap-2">
                        <span>{icon}</span>
                        <span class="card-title text-base">{name}</span>
                    </div>
                    <div class="flex gap-1" on:click=move |ev| ev.stop_propagation()>
                        {on_pin.map(|cb| {
                            let cid = cid_pin.clone();
                            view! {
                                <button
                                    type="button"
                                    class=move || if is_pinned { "btn btn-ghost btn-xs text-warning" } else { "btn btn-ghost btn-xs" }
                                    on:click=move |_| cb.run(cid.clone())
                                >
                                    {"📌"}
                                </button>
                            }
                        })}
                        {on_delete.map(|cb| {
                            let cid = cid_del.clone();
                            view! {
                                <button
                                    type="button"
                                    class="btn btn-ghost btn-xs text-error"
                                    on:click=move |_| cb.run(cid.clone())
                                >
                                    {"✕"}
                                </button>
                            }
                        })}
                    </div>
                </div>
            </div>
        </div>
    };

    match (dnd_state, on_nest_drop) {
        (Some(state), Some(cb)) => {
            let id_handle = cid.clone();
            let id_shell = cid.clone();
            let id_surface = cid.clone();
            let id_nest = cid;
            view! {
                <DragShell dnd_state=state dragged_id=id_shell>
                    <DragHandleButton dnd_state=state kind=EntityKind::Container dragged_id=id_handle aria_label="Przeciągnij folder" />
                    <DragSurface dnd_state=state dragged_id=id_surface nest_target_id=id_nest on_drop=cb>
                        {body}
                    </DragSurface>
                </DragShell>
            }.into_any()
        }
        _ => body.into_any(),
    }
}
