//! The list page.
//!
//! One resource feeds the page and refetches on every change of the list, its items, tags or
//! comments (including live updates from other tabs). The page never rebuilds itself from it:
//! the first child of the `<Transition>` patches the data into [`ListPageState`], and
//! everything below is created once and reads small memos, stores and signals of that state
//! (see `state.rs`). Open editors, focus and typed text therefore survive a refetch.
//! Keep reads of the resource out of the closures that wrap the rest of the page.

mod header;
mod items;
mod state;

use std::collections::HashMap;

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use kartoteka_shared::ChangeTopic;

use self::header::{ConfirmAction, ConfirmRequest, ListHeader};
use self::items::ListItems;
use self::state::ListPageState;
use crate::components::comments::CommentSection;
use crate::components::common::confirm_modal::{ConfirmModal, ConfirmVariant};
use crate::components::common::delete_entity_modal::{DeleteEntityModal, DeleteTarget};
use crate::components::common::dnd::DetachDropZone;
use crate::components::common::loading::LoadingSpinner;
use crate::components::items::item_composer::{AddItemDraft, ItemComposer};
use crate::components::items::item_list::ItemDnd;
use crate::components::items::item_view::ItemCaps;
use crate::components::lists::{add_input::AddInput, sublist_section::SublistSection};
use crate::components::tags::tag_filter_bar::TagFilterBar;
use crate::components::tags::tag_list::TagList;
use crate::components::ui::SectionHeader;
use crate::server_fns::items::{get_list_data, reorder_items, set_item_placement};
use crate::server_fns::lists::{archive_list, create_list, get_all_lists, move_list, reset_list};
use crate::server_fns::tags::{assign_tag_to_list, remove_tag_from_list};
use crate::state::dnd::{
    DndState, EntityKind, ItemDndState, ItemDropPlan, ItemDropTarget, build_item_drop_plan,
};
use crate::state::invalidation::{mutate, mutate_with, use_invalidation};
use crate::state::item_store::tracked_items;

#[component]
pub fn ListPage() -> impl IntoView {
    let params = use_params_map();
    let list_id = Signal::derive(move || params.read().get("id").unwrap_or_default());

    let navigate = leptos_router::hooks::use_navigate();
    // Separate handle for the shared delete flow, so the archive/reset modal keeps `navigate`.
    let nav_delete = navigate.clone();
    let invalidation = use_invalidation();

    let show_completed = RwSignal::new(true);
    let active_tag: RwSignal<Option<String>> = RwSignal::new(None);

    let data_res = Resource::new(
        move || {
            (
                list_id.get(),
                invalidation.track(&[
                    ChangeTopic::Lists,
                    ChangeTopic::Items,
                    ChangeTopic::Tags,
                    ChangeTopic::Comments,
                ]),
            )
        },
        |(id, _)| get_list_data(id),
    );
    let all_lists_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::Lists]),
        |_| get_all_lists(),
    );

    let state = ListPageState::new();

    let on_tag_toggle = Callback::new(move |tag_id: String| {
        let has_tag = state
            .list_tag_ids
            .with_untracked(|ids| ids.contains(&tag_id));
        let lid = list_id.get_untracked();
        let result = async move {
            if has_tag {
                remove_tag_from_list(lid, tag_id).await
            } else {
                assign_tag_to_list(lid, tag_id).await
            }
        };
        mutate(result, &[ChangeTopic::Lists, ChangeTopic::Tags]);
    });

    let on_create_sublist = Callback::new(move |name: String| {
        use kartoteka_shared::types::CreateListRequest;
        let req = CreateListRequest {
            name,
            list_type: None,
            icon: None,
            description: None,
            container_id: None,
            parent_list_id: Some(list_id.get_untracked()),
            features: vec![],
            color: None,
            inherit_from_parent: true,
        };
        mutate(
            create_list(req),
            &[ChangeTopic::Lists, ChangeTopic::Containers],
        );
    });

    let (adding_sublist, set_adding_sublist) = signal(false);

    // The add-item form lives as long as the page, but a different list starts with an
    // empty draft.
    let add_item_draft = AddItemDraft::new();
    Effect::new(move |previous: Option<String>| {
        let current = list_id.get();
        if previous.is_some_and(|previous| previous != current) {
            add_item_draft.clear();
        }
        current
    });

    let confirm = ConfirmRequest {
        action: RwSignal::new(None),
        list_name: RwSignal::new(String::new()),
    };
    // Delete goes through the shared preview + confirmation flow.
    let delete_target: RwSignal<Option<DeleteTarget>> = RwSignal::new(None);
    let redirect_after_delete = move || {
        state
            .meta
            .with_untracked(|m| {
                m.as_ref().map(
                    |m| match (m.parent_list_id.as_deref(), m.container_id.as_deref()) {
                        (Some(pid), _) => format!("/lists/{pid}"),
                        (_, Some(cid)) => format!("/containers/{cid}"),
                        _ => "/".to_string(),
                    },
                )
            })
            .unwrap_or_else(|| "/".to_string())
    };

    // ── DnD state + callbacks ─────────────────────────────────────
    let dnd_state: RwSignal<DndState> = RwSignal::new(DndState::default());
    let item_dnd_state: RwSignal<ItemDndState> = RwSignal::new(ItemDndState::default());

    // Used by reorder + cross-list move plan builder; rebuilt each call so it reflects the
    // current rows. Sublists' rows are not included: for a cross-list drop the plan's Move
    // branch is fine without full maps for the target list (server resolves).
    let build_ids_map = move || -> HashMap<String, Vec<String>> {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        if let Some(id) = state
            .meta
            .with_untracked(|m| m.as_ref().map(|m| m.id.clone()))
        {
            let mut main = tracked_items(state.items);
            main.sort_by(|a, b| {
                a.completed
                    .cmp(&b.completed)
                    .then(a.position.cmp(&b.position))
            });
            map.insert(id, main.into_iter().map(|i| i.id).collect());
        }
        map
    };

    let on_item_drop = Callback::new(move |target: ItemDropTarget| {
        let Some(dragged) = item_dnd_state.with_untracked(|s| s.dragged_item.clone()) else {
            return;
        };
        let ids_map = build_ids_map();
        let Some(plan) = build_item_drop_plan(&ids_map, &dragged, &target) else {
            return;
        };
        mutate(
            async move {
                match plan {
                    ItemDropPlan::Reorder {
                        list_id: lid,
                        item_ids,
                    } => reorder_items(lid, item_ids).await.map(|_| ()),
                    ItemDropPlan::Move {
                        item_id,
                        target_list_id,
                        before_item_id,
                        ..
                    } => set_item_placement(item_id, target_list_id, before_item_id).await,
                }
            },
            &[ChangeTopic::Items, ChangeTopic::Lists],
        );
    });
    let item_dnd = ItemDnd {
        state: item_dnd_state,
        on_drop: on_item_drop,
    };

    let features = state.field(|m| m.features.clone());
    let composer_caps = Memo::new(move |_| features.with(|f| ItemCaps::from_features(f)));
    let sublist_ids = Memo::new(move |_| {
        state
            .sublists
            .with(|subs| subs.iter().map(|s| s.id.clone()).collect::<Vec<_>>())
    });
    let sublist_count = Memo::new(move |_| sublist_ids.with(Vec::len));
    let has_sublists = Memo::new(move |_| sublist_count.get() > 0);

    let detach_visible = Signal::derive(move || {
        dnd_state.with(|s| {
            s.dragged.as_ref().is_some_and(|d| {
                d.kind == EntityKind::List && sublist_ids.with(|ids| ids.iter().any(|x| x == &d.id))
            })
        })
    });
    let on_sublist_detach = Callback::new(move |_| {
        let Some(id) = dnd_state.with_untracked(|s| s.dragged.as_ref().map(|d| d.id.clone()))
        else {
            return;
        };
        let container = state
            .meta
            .with_untracked(|m| m.as_ref().and_then(|m| m.container_id.clone()));
        mutate(
            move_list(id, container, None),
            &[ChangeTopic::Lists, ChangeTopic::Containers],
        );
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            // Transition: a refetch keeps the current page on screen instead of the spinner.
            <Transition fallback=|| view! { <LoadingSpinner/> }>
                // Patch the page state first: this closure runs before the page below in the
                // same render pass, on the server and in the browser.
                {move || {
                    let all_lists = all_lists_res.get();
                    match data_res.get() {
                        Some(Ok(data)) => {
                            state.patch(data);
                            if let Some(Ok(lists)) = all_lists {
                                state.patch_all_lists(lists);
                            }
                            ().into_any()
                        }
                        Some(Err(e)) => {
                            state.set_failed();
                            view! { <p class="text-error">"Błąd: " {e.to_string()}</p> }.into_any()
                        }
                        None => ().into_any(),
                    }
                }}

                <Show when=move || state.loaded.get()>
                    <div>
                        <ListHeader
                            state=state
                            list_id=list_id
                            confirm=confirm
                            delete_target=delete_target
                        />

                        // Tags
                        <div class="mb-4" data-testid="list-tags-section">
                            {move || view! {
                                <TagList
                                    all_tags=state.all_tags.get()
                                    selected_tag_ids=state.list_tag_ids.get()
                                    on_toggle=on_tag_toggle
                                />
                            }}
                        </div>

                        // Tag filter bar (shown only when items have tags)
                        {move || view! {
                            <TagFilterBar
                                all_tags=state.all_tags.get()
                                item_tag_links=state.item_tag_links.get()
                                active_tag=active_tag
                            />
                        }}

                        // Add item form. The draft is owned by the page, so the text survives
                        // the form being re-created by a refetch.
                        <div class="mb-4">
                            <ItemComposer
                                list_id=list_id
                                caps=composer_caps
                                draft=add_item_draft
                            />
                        </div>

                        // Sublist detach zone — visible when dragging a sublist of this list.
                        <DetachDropZone
                            dnd_state=dnd_state
                            visible=detach_visible
                            on_drop=on_sublist_detach
                            label="Upuść tutaj, aby odpiąć podlistę"
                        />

                        // Sublists section
                        <div class="mb-4">
                            <Show when=move || has_sublists.get()>
                                {move || view! {
                                    <SectionHeader
                                        title="Podlisty"
                                        count=sublist_count.get()
                                        icon="list"
                                        class="mb-2"
                                    />
                                }}
                                <div class="flex flex-col gap-2 mb-2">
                                    <For each=move || sublist_ids.get() key=|id| id.clone() let:sid>
                                        <SublistSection
                                            store=state.sublist_store(&sid)
                                            id=sid
                                            sublists=Signal::from(state.sublists)
                                            all_lists=Signal::from(state.all_lists)
                                            dnd_state=dnd_state
                                            item_dnd=item_dnd
                                        />
                                    </For>
                                </div>
                            </Show>
                            // Add sublist button
                            {move || if adding_sublist.get() {
                                view! {
                                    <AddInput
                                        placeholder=Signal::derive(|| "Nazwa podlisty...".to_string())
                                        button_label=Signal::derive(|| "Utwórz".to_string())
                                        on_submit=Callback::new(move |name: String| {
                                            set_adding_sublist.set(false);
                                            on_create_sublist.run(name);
                                        })
                                        testid="add-sublist-input"
                                    />
                                }.into_any()
                            } else {
                                view! {
                                    <button
                                        type="button"
                                        class="btn btn-ghost btn-sm"
                                        data-testid="add-sublist-button"
                                        on:click=move |_| set_adding_sublist.set(true)
                                    >
                                        "+ Dodaj podlistę"
                                    </button>
                                }.into_any()
                            }}
                        </div>

                        // Items list
                        <ListItems
                            state=state
                            show_completed=show_completed
                            active_tag=active_tag
                            item_dnd=item_dnd
                        />

                        // Comments
                        <CommentSection
                            entity_type="list"
                            entity_id=list_id
                            initial_payload=state.comments.get_untracked()
                            live_payload=state.comments
                        />
                    </div>
                </Show>
            </Transition>

            <DeleteEntityModal
                target=delete_target
                on_deleted=Callback::new(move |_| {
                    nav_delete(&redirect_after_delete(), Default::default());
                })
            />

            {move || {
                let action = confirm.action.get()?;
                let name = confirm.list_name.get();
                let (title, message, label, variant) = match action {
                    ConfirmAction::Archive => (
                        "Archiwizuj listę",
                        format!("Czy archiwizować listę \"{}\"?", name),
                        "Archiwizuj",
                        ConfirmVariant::Warning,
                    ),
                    ConfirmAction::Reset => (
                        "Resetuj listę",
                        format!("Odznaczyć wszystkie ukończone elementy na liście \"{}\"?", name),
                        "Resetuj",
                        ConfirmVariant::Warning,
                    ),
                };
                let nav = navigate.clone();
                let close = Callback::new(move |_| confirm.action.set(None));
                Some(view! {
                    <ConfirmModal
                        open=Signal::derive(move || confirm.action.get().is_some())
                        title=title.to_string()
                        message=message
                        confirm_label=label.to_string()
                        variant=variant
                        on_close=close
                        on_confirm=Callback::new(move |_| {
                            confirm.action.set(None);
                            let lid = list_id.get_untracked();
                            let nav2 = nav.clone();
                            match action {
                                ConfirmAction::Archive => mutate_with(
                                    archive_list(lid),
                                    &[ChangeTopic::Lists, ChangeTopic::Items],
                                    move |_| nav2(&redirect_after_delete(), Default::default()),
                                ),
                                ConfirmAction::Reset => {
                                    mutate(reset_list(lid), &[ChangeTopic::Items])
                                }
                            }
                        })
                    />
                })
            }}
        </div>
    }
}
