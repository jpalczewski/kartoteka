use kartoteka_shared::types::{List, Tag};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::components::common::dnd::{DragHandleButton, DragShell, DragSurface};
use crate::components::tags::tag_list::TagList;
use crate::state::dnd::{DndState, DropTarget, EntityKind};

pub fn list_type_icon(lt: &str) -> &'static str {
    match lt {
        "checklist" => "✅",
        "shopping" => "🛒",
        "schedule" | "habits" => "📅",
        "log" => "⏱",
        "notes" => "📓",
        _ => "📝",
    }
}

#[component]
pub fn ListCard(
    list: List,
    #[prop(default = vec![])] all_tags: Vec<Tag>,
    #[prop(default = vec![])] list_tag_ids: Vec<String>,
    #[prop(optional)] on_tag_toggle: Option<Callback<String>>,
    #[prop(optional)] on_delete: Option<Callback<String>>,
    /// If provided, the card gains a drag handle and becomes a nest drop target.
    /// The callback fires with `DropTarget::Nest(list.id)` when something is
    /// dropped onto the card body.
    #[prop(optional)]
    dnd_state: Option<RwSignal<DndState>>,
    #[prop(optional)] on_nest_drop: Option<Callback<DropTarget>>,
) -> impl IntoView {
    let href = format!("/lists/{}", list.id);
    let icon = list_type_icon(&list.list_type.clone());
    let navigate = use_navigate();
    let href_nav = href.clone();

    let list_id = list.id.clone();
    let list_id_del = list.id.clone();
    let list_name = list.name.clone();

    let body = view! {
        <div
            class="card bg-base-200 border border-base-300 cursor-pointer card-neon relative flex-1"
            data-testid="list-card"
            on:click=move |_| { navigate(&href_nav, Default::default()); }
        >
            {on_delete.map(|cb| {
                let lid = list_id_del.clone();
                view! {
                    <button
                        type="button"
                        class="btn btn-ghost btn-xs btn-circle absolute top-2 right-2 z-10 text-error"
                        on:click=move |ev| {
                            ev.stop_propagation();
                            cb.run(lid.clone());
                        }
                    >
                        {"✕"}
                    </button>
                }
            })}
            <div class="card-body p-4">
                <div class="flex items-center gap-2">
                    <span>{icon}</span>
                    <span class="card-title text-base" data-testid="list-card-title">{list_name}</span>
                </div>
                {match on_tag_toggle {
                    Some(cb) => view! {
                        <TagList
                            all_tags=all_tags.clone()
                            selected_tag_ids=list_tag_ids.clone()
                            on_toggle=cb
                        />
                    }.into_any(),
                    None => view! {
                        <TagList
                            all_tags=all_tags.clone()
                            selected_tag_ids=list_tag_ids.clone()
                        />
                    }.into_any(),
                }}
            </div>
        </div>
    };

    match (dnd_state, on_nest_drop) {
        (Some(state), Some(cb)) => {
            let id_handle = list_id.clone();
            let id_shell = list_id.clone();
            let id_surface = list_id.clone();
            let id_nest = list_id;
            view! {
                <DragShell dnd_state=state dragged_id=id_shell>
                    <DragHandleButton dnd_state=state kind=EntityKind::List dragged_id=id_handle aria_label="Przeciągnij listę" />
                    <DragSurface dnd_state=state dragged_id=id_surface nest_target_id=id_nest on_drop=cb>
                        {body}
                    </DragSurface>
                </DragShell>
            }.into_any()
        }
        _ => body.into_any(),
    }
}
