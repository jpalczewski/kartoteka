use kartoteka_shared::default_list_icon;
use kartoteka_shared::types::{List, Tag};
use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::hooks::use_navigate;

use crate::components::common::dnd::{DragHandleButton, DragShell, DragSurface};
use crate::components::tags::tag_list::TagList;
use crate::components::ui::{ActionMenu, ActionMenuItem, EntityIcon};
use crate::state::dnd::{DndState, DropTarget, EntityKind};

#[component]
pub fn ListCard(
    list: List,
    #[prop(default = vec![])] all_tags: Vec<Tag>,
    #[prop(default = vec![])] list_tag_ids: Vec<String>,
    #[prop(optional)] on_tag_toggle: Option<Callback<String>>,
    #[prop(optional)] on_delete: Option<Callback<String>>,
    #[prop(optional)] on_archive: Option<Callback<String>>,
    #[prop(optional)] dnd_state: Option<RwSignal<DndState>>,
    #[prop(optional)] on_nest_drop: Option<Callback<DropTarget>>,
) -> impl IntoView {
    let href = format!("/lists/{}", list.id);
    let icon = list.icon.clone();
    let icon_fallback = default_list_icon(&list.list_type);
    let navigate = use_navigate();
    let href_nav = href.clone();

    let list_id = list.id.clone();
    let list_id_del = list.id.clone();
    let list_id_archive = list.id.clone();
    let list_name = list.name.clone();

    let list_archived = list.archived;
    let menu_open = RwSignal::new(false);
    let mut menu_items = Vec::new();
    if let Some(cb) = on_archive {
        let lid = list_id_archive.clone();
        menu_items.push(
            ActionMenuItem::new(
                "archive",
                if list_archived {
                    move_tr!("lists-restore-aria")
                } else {
                    move_tr!("lists-archive-aria")
                },
                Callback::new(move |_: ()| cb.run(lid.clone())),
            )
            .testid("list-action-archive"),
        );
    }
    if let Some(cb) = on_delete {
        let lid = list_id_del.clone();
        menu_items.push(
            ActionMenuItem::new(
                "trash",
                move_tr!("lists-header-delete-button"),
                Callback::new(move |_: ()| cb.run(lid.clone())),
            )
            .danger()
            .testid("list-card-delete"),
        );
    }

    let body = view! {
        <div
            class="card bg-base-200 border border-base-300 cursor-pointer card-neon relative flex-1"
            data-testid="list-card"
            on:click=move |_| { navigate(&href_nav, Default::default()); }
        >

            <div class="card-body p-4">
                <div class="flex items-center gap-2">
                    <EntityIcon raw=icon fallback=icon_fallback />
                    <span class="card-title text-base" data-testid="list-card-title">{list_name}</span>
                    {(!menu_items.is_empty()).then(move || view! {
                        <div class="ml-auto">
                            <ActionMenu
                                items=menu_items
                                open=menu_open
                                label=move_tr!("common-actions")
                                testid=Some("list-card-menu")
                            />
                        </div>
                    })}
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
