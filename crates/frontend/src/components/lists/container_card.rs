use kartoteka_shared::{default_container_icon, types::Container};
use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::hooks::use_navigate;

use crate::components::common::dnd::{DragHandleButton, DragShell, DragSurface};
use crate::components::ui::{ActionMenu, ActionMenuItem, EntityIcon};
use crate::state::dnd::{DndState, DropTarget, EntityKind};

#[component]
pub fn ContainerCard(
    container: Container,
    #[prop(optional)] on_delete: Option<Callback<String>>,
    #[prop(optional)] on_archive: Option<Callback<String>>,
    #[prop(optional)] on_pin: Option<Callback<String>>,
    #[prop(optional)] dnd_state: Option<RwSignal<DndState>>,
    #[prop(optional)] on_nest_drop: Option<Callback<DropTarget>>,
) -> impl IntoView {
    let href = format!("/containers/{}", container.id);
    let icon = container.icon.clone();
    let is_project = container.status.is_some();
    let is_pinned = container.pinned;
    let navigate = use_navigate();
    let href_nav = href.clone();

    let cid = container.id.clone();
    let cid_del = container.id.clone();
    let cid_archive = container.id.clone();
    let cid_pin = container.id.clone();
    let name = container.name.clone();
    let ancestor_path = (!container.ancestors.is_empty()).then(|| {
        container
            .ancestors
            .iter()
            .map(|(_, n)| n.as_str())
            .collect::<Vec<_>>()
            .join(" / ")
    });

    let menu_open = RwSignal::new(false);
    let mut menu_items = Vec::new();
    if let Some(cb) = on_pin {
        let cid = cid_pin.clone();
        menu_items.push(
            ActionMenuItem::new(
                if is_pinned { "pin-off" } else { "pin" },
                if is_pinned {
                    move_tr!("lists-unpin")
                } else {
                    move_tr!("lists-pin")
                },
                Callback::new(move |_: ()| cb.run(cid.clone())),
            )
            .testid("container-action-pin"),
        );
    }
    if let Some(cb) = on_archive {
        let cid = cid_archive.clone();
        menu_items.push(
            ActionMenuItem::new(
                "archive",
                move_tr!("lists-header-archive-button"),
                Callback::new(move |_: ()| cb.run(cid.clone())),
            )
            .testid("container-action-archive"),
        );
    }
    if let Some(cb) = on_delete {
        let cid = cid_del.clone();
        menu_items.push(
            ActionMenuItem::new(
                "trash",
                move_tr!("lists-header-delete-button"),
                Callback::new(move |_: ()| cb.run(cid.clone())),
            )
            .danger()
            .testid("container-card-delete"),
        );
    }

    let body = view! {
        <div
            class="card bg-base-200 border border-base-300 cursor-pointer card-neon relative flex-1"
            data-testid="container-card"
            on:click=move |_| { navigate(&href_nav, Default::default()); }
        >
            <div class="card-body p-4">
                <div class="flex items-center justify-between">
                    <div class="flex items-center gap-2 min-w-0">
                        <EntityIcon raw=icon fallback=default_container_icon(is_project) />
                        <div class="min-w-0">
                            <span class="card-title text-base">{name}</span>
                            {ancestor_path.map(|path| view! { <p class="text-xs text-base-content/50 truncate">{path}</p> })}
                        </div>
                    </div>
                    {(!menu_items.is_empty()).then(move || view! {
                        <ActionMenu
                            items=menu_items
                            open=menu_open
                            label=move_tr!("common-actions")
                            testid=Some("container-card-menu")
                        />
                    })}
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
