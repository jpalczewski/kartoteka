use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::hooks::use_params_map;

use kartoteka_shared::{
    FEATURE_CHECKLIST, FEATURE_DEADLINES, FEATURE_LOCATION, FEATURE_QUANTITY, FEATURE_TIME_TRACKING,
};

use crate::app::{ToastContext, ToastKind};
use crate::components::comments::CommentSection;
use crate::components::common::breadcrumbs::Breadcrumbs;
use crate::components::common::confirm_modal::{ConfirmModal, ConfirmVariant};
use crate::components::common::dnd::{DetachDropZone, ItemDropTargetMarker};
use crate::components::common::editable_text::EditableText;
use crate::components::common::loading::LoadingSpinner;
use crate::components::items::item_row::ItemRow;
use crate::components::lists::{
    add_input::AddInput, add_item_input::AddItemInput, deadlines_config::DeadlinesConfig,
    list_card::list_type_icon, sublist_section::SublistSection,
};
use crate::components::tags::tag_filter_bar::TagFilterBar;
use crate::components::tags::tag_list::TagList;
use crate::context::GlobalRefresh;
use crate::server_fns::items::{
    delete_item, get_list_data, move_item, reorder_items, set_item_placement, toggle_item,
    update_actual_quantity, update_item_dates, update_item_description,
};
use crate::server_fns::lists::{
    archive_list, create_list, delete_list, get_all_lists, move_list, pin_list, rename_list,
    reset_list, update_list_features,
};
use crate::server_fns::tags::{
    assign_tag_to_item, assign_tag_to_list, get_all_tags, get_list_tag_links, remove_tag_from_item,
    remove_tag_from_list,
};
use crate::state::dnd::{
    DndState, EntityKind, ItemDndState, ItemDropPlan, ItemDropTarget, build_item_drop_plan,
};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq)]
enum ConfirmAction {
    Delete,
    Archive,
    Reset,
}

#[component]
pub fn ListPage() -> impl IntoView {
    let params = use_params_map();
    let list_id = move || params.read().get("id").unwrap_or_default();

    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let global_refresh = use_context::<GlobalRefresh>().expect("GlobalRefresh missing");
    let navigate = leptos_router::hooks::use_navigate();

    let (refresh, set_refresh) = signal(0u32);
    let (show_completed, set_show_completed) = signal(true);
    let active_tag: RwSignal<Option<String>> = RwSignal::new(None);

    let data_res = Resource::new(
        move || (list_id(), refresh.get(), global_refresh.get()),
        |(id, _, _)| get_list_data(id),
    );

    // Derived reactive counts — read from data_res each render so they update
    // every time the resource refetches (e.g. after toggle/reset).
    let completed_count = Signal::derive(move || {
        data_res
            .get()
            .and_then(|r| r.ok())
            .map(|d| d.items.iter().filter(|i| i.completed).count())
            .unwrap_or(0)
    });
    let total = Signal::derive(move || {
        data_res
            .get()
            .and_then(|r| r.ok())
            .map(|d| d.items.len())
            .unwrap_or(0)
    });

    let all_lists_res = Resource::new(
        move || (refresh.get(), global_refresh.get()),
        |_| get_all_lists(),
    );

    let tag_res = Resource::new(
        move || (list_id(), refresh.get(), global_refresh.get()),
        |(lid, _, _)| async move {
            let tags = get_all_tags().await?;
            let links = get_list_tag_links().await?;
            let tag_ids: Vec<String> = links
                .into_iter()
                .filter(|l| l.list_id == lid)
                .map(|l| l.tag_id)
                .collect();
            Ok::<(Vec<kartoteka_shared::types::Tag>, Vec<String>), ServerFnError>((tags, tag_ids))
        },
    );

    let on_tag_toggle = Callback::new(move |tag_id: String| {
        let lid = list_id();
        let current_tag_ids = tag_res
            .get()
            .and_then(|r| r.ok())
            .map(|(_, ids)| ids)
            .unwrap_or_default();
        let has_tag = current_tag_ids.contains(&tag_id);
        leptos::task::spawn_local(async move {
            let result = if has_tag {
                remove_tag_from_list(lid, tag_id).await
            } else {
                assign_tag_to_list(lid, tag_id).await
            };
            if let Err(e) = result {
                toast.push(e.to_string(), ToastKind::Error);
            }
            set_refresh.update(|n| *n += 1);
        });
    });

    // ── Mutation callbacks ─────────────────────────────────────────

    let on_toggle_item = Callback::new(move |item_id: String| {
        leptos::task::spawn_local(async move {
            match toggle_item(item_id).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_delete_item = Callback::new(move |item_id: String| {
        leptos::task::spawn_local(async move {
            match delete_item(item_id).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_description_save = Callback::new(move |(item_id, desc): (String, String)| {
        leptos::task::spawn_local(async move {
            match update_item_description(item_id, desc).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_quantity_change = Callback::new(move |(item_id, new_actual): (String, i32)| {
        leptos::task::spawn_local(async move {
            match update_actual_quantity(item_id, new_actual).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_move_item = Callback::new(move |(item_id, target_list_id): (String, String)| {
        leptos::task::spawn_local(async move {
            match move_item(item_id, target_list_id).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_date_save = Callback::new(
        move |payload: crate::components::items::item_row::DateSavePayload| {
            let (item_id, start, start_time, deadline, deadline_time, hard) = payload;
            leptos::task::spawn_local(async move {
                match update_item_dates(item_id, start, start_time, deadline, deadline_time, hard)
                    .await
                {
                    Ok(_) => set_refresh.update(|n| *n += 1),
                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                }
            });
        },
    );

    let on_item_tag_toggle = Callback::new(move |(item_id, tag_id): (String, String)| {
        let links = data_res
            .get()
            .and_then(|r| r.ok())
            .map(|d| d.item_tag_links)
            .unwrap_or_default();
        let is_assigned = links
            .iter()
            .any(|l| l.item_id == item_id && l.tag_id == tag_id);
        leptos::task::spawn_local(async move {
            let result = if is_assigned {
                remove_tag_from_item(item_id, tag_id).await
            } else {
                assign_tag_to_item(item_id, tag_id).await
            };
            match result {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_create_sublist = Callback::new(move |name: String| {
        use kartoteka_shared::types::CreateListRequest;
        let parent_id = list_id();
        leptos::task::spawn_local(async move {
            let req = CreateListRequest {
                name,
                list_type: None,
                icon: None,
                description: None,
                container_id: None,
                parent_list_id: Some(parent_id),
                features: vec![],
            };
            match create_list(req).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let (adding_sublist, set_adding_sublist) = signal(false);

    let confirm_action: RwSignal<Option<ConfirmAction>> = RwSignal::new(None);
    let confirm_list_name: RwSignal<String> = RwSignal::new(String::new());

    // ── DnD state + callbacks ─────────────────────────────────────
    let dnd_state: RwSignal<DndState> = RwSignal::new(DndState::default());
    let item_dnd_state: RwSignal<ItemDndState> = RwSignal::new(ItemDndState::default());

    // Used by reorder + cross-list move plan builder; rebuilt each call so it
    // reflects current data_res snapshot.
    let build_ids_map = move || -> HashMap<String, Vec<String>> {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        if let Some(Ok(data)) = data_res.get() {
            let mut main: Vec<kartoteka_shared::types::Item> = data.items.clone();
            main.sort_by(|a, b| {
                a.completed
                    .cmp(&b.completed)
                    .then(a.position.cmp(&b.position))
            });
            map.insert(
                data.list.id.clone(),
                main.into_iter().map(|i| i.id).collect(),
            );
            // Sublists' items aren't in data.items — SublistSection fetches its own.
            // For cross-list reorder, pages only know main list ids; plan's Move
            // branch is fine without full maps for the target list (server resolves).
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
        leptos::task::spawn_local(async move {
            let result = match plan {
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
            };
            match result {
                Ok(()) => {
                    set_refresh.update(|n| *n += 1);
                    global_refresh.bump();
                }
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <Transition fallback=|| view! { <LoadingSpinner/> }>
                {move || data_res.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="text-error">"Błąd: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(data) => {
                        let icon = list_type_icon(&data.list.list_type);
                        let list_name = data.list.name.clone();
                        let list_name_for_desc = list_name.clone();
                        let list_name_for_reset = list_name.clone();
                        let list_name_for_archive = list_name.clone();
                        let list_name_for_delete = list_name.clone();
                        let list_description = data.list.description.clone();
                        let list_pinned = data.list.pinned;
                        let list_archived = data.list.archived;
                        let created_at_local = data.created_at_local.clone();
                        let all_items = data.items.clone();
                        let sublists = data.sublists.clone();
                        let parent_container_id = data.list.container_id.clone();
                        let current_list_id = data.list.id.clone();
                        let sublist_ids: Vec<String> = sublists.iter().map(|s| s.id.clone()).collect();
                        let all_lists_vec = all_lists_res.get().and_then(|r| r.ok()).unwrap_or_default();
                        let move_targets_all: Vec<(String, String)> = all_lists_vec
                            .iter()
                            .filter(|l| l.id != current_list_id)
                            .map(|l| (l.id.clone(), l.name.clone()))
                            .collect();
                        let targets_for_items = move_targets_all.clone();
                        let item_tag_links = data.item_tag_links.clone();
                        let item_tag_links_filter = data.item_tag_links.clone();
                        let all_tags_for_filter = data.all_tags.clone();
                        let all_tags_for_rows = data.all_tags.clone();
                        let all_tags_for_selector = data.all_tags.clone();
                        let today_date = data.today_date.clone();
                        let breadcrumb_crumbs = if let Some(ref cname) = data.container_name {
                            let cid = data.list.container_id.clone().unwrap_or_default();
                            vec![(format!("/containers/{cid}"), cname.clone())]
                        } else {
                            vec![]
                        };
                        let breadcrumb_current = list_name.clone();
                        let current_features: Vec<String> = data
                            .list
                            .features
                            .iter()
                            .map(|f| f.feature_name.clone())
                            .collect();
                        let has_quantity = current_features.iter().any(|f| f == FEATURE_QUANTITY);
                        let has_deadlines = current_features.iter().any(|f| f == FEATURE_DEADLINES);
                        let has_location = current_features.iter().any(|f| f == FEATURE_LOCATION);
                        let has_checklist = current_features.iter().any(|f| f == FEATURE_CHECKLIST);
                        let has_time_tracking =
                            current_features.iter().any(|f| f == FEATURE_TIME_TRACKING);
                        let deadlines_config = data
                            .list
                            .features
                            .iter()
                            .find(|f| f.feature_name == FEATURE_DEADLINES)
                            .map(|f| f.config.clone())
                            .unwrap_or_else(|| serde_json::json!({}));

                        view! {
                            <div>
                                <Breadcrumbs crumbs=breadcrumb_crumbs current=breadcrumb_current />

                                // Header
                                <div class="mb-1 flex items-center gap-2">
                                    <span class="text-2xl">{icon}</span>
                                    <EditableText
                                        value=list_name.clone()
                                        on_save=Callback::new(move |new_name: String| {
                                            let lid = list_id();
                                            leptos::task::spawn_local(async move {
                                                match rename_list(lid, new_name, None).await {
                                                    Ok(_) => set_refresh.update(|n| *n += 1),
                                                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                                }
                                            });
                                        })
                                        class="text-2xl font-bold cursor-pointer hover:underline decoration-dotted"
                                        testid="list-name-heading"
                                    />
                                    // Dropdown at the end, pushed right via ml-auto
                                    <div class="dropdown dropdown-end ml-auto">
                                        <div tabindex="0" role="button" class="btn btn-ghost btn-sm btn-circle" data-testid="list-actions-btn">
                                            "⋮"
                                        </div>
                                        <ul tabindex="0" class="dropdown-content menu bg-base-100 rounded-box z-50 w-52 p-2 shadow-lg border border-base-300">
                                            <li>
                                                <button
                                                    type="button"
                                                    data-testid="action-pin"
                                                    on:click=move |_| {
                                                        let lid = list_id();
                                                        leptos::task::spawn_local(async move {
                                                            match pin_list(lid).await {
                                                                Ok(_) => set_refresh.update(|n| *n += 1),
                                                                Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                                            }
                                                        });
                                                    }
                                                >
                                                    {if list_pinned { "📌 Odepnij" } else { "📌 Przypnij" }}
                                                </button>
                                            </li>
                                            <li>
                                                <button
                                                    type="button"
                                                    data-testid="action-reset"
                                                    on:click=move |_| {
                                                        confirm_list_name.set(list_name_for_reset.clone());
                                                        confirm_action.set(Some(ConfirmAction::Reset));
                                                    }
                                                >
                                                    "↺ Resetuj ukończone"
                                                </button>
                                            </li>
                                            <li class="menu-title">
                                                <span class="text-xs uppercase tracking-wide opacity-60">"Funkcje"</span>
                                            </li>
                                            {
                                                let make_toggle = {
                                                    let feats = current_features.clone();
                                                    move |feature: &'static str| {
                                                        let f0 = feats.clone();
                                                        move |ev: leptos::ev::Event| {
                                                            let checked = event_target_checked(&ev);
                                                            let mut f = f0.clone();
                                                            if checked { f.push(feature.to_string()); }
                                                            else { f.retain(|x| x != feature); }
                                                            let lid = list_id();
                                                            leptos::task::spawn_local(async move {
                                                                match update_list_features(lid, f).await {
                                                                    Ok(_) => set_refresh.update(|n| *n += 1),
                                                                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                                                }
                                                            });
                                                        }
                                                    }
                                                };
                                                view! {
                                                    <li>
                                                        <label class="flex items-center gap-2 cursor-pointer">
                                                            <input
                                                                type="checkbox"
                                                                class="checkbox checkbox-xs"
                                                                prop:checked=has_quantity
                                                                on:change=make_toggle(FEATURE_QUANTITY)
                                                            />
                                                            "Ilości"
                                                        </label>
                                                    </li>
                                                    <li>
                                                        <label class="flex items-center gap-2 cursor-pointer">
                                                            <input
                                                                type="checkbox"
                                                                class="checkbox checkbox-xs"
                                                                prop:checked=has_deadlines
                                                                on:change=make_toggle(FEATURE_DEADLINES)
                                                            />
                                                            "Terminy"
                                                        </label>
                                                    </li>
                                                    {if has_deadlines {
                                                        let cfg = deadlines_config.clone();
                                                        let lid = current_list_id.clone();
                                                        view! {
                                                            <DeadlinesConfig
                                                                list_id=lid
                                                                config=cfg
                                                                on_changed=Callback::new(move |_| set_refresh.update(|n| *n += 1))
                                                            />
                                                        }.into_any()
                                                    } else {
                                                        view! {}.into_any()
                                                    }}
                                                    <li>
                                                        <label class="flex items-center gap-2 cursor-pointer">
                                                            <input
                                                                type="checkbox"
                                                                class="checkbox checkbox-xs"
                                                                prop:checked=has_location
                                                                on:change=make_toggle(FEATURE_LOCATION)
                                                            />
                                                            "📍 " {move_tr!("lists-feature-location")}
                                                        </label>
                                                    </li>
                                                    <li>
                                                        <label class="flex items-center gap-2 cursor-pointer">
                                                            <input
                                                                type="checkbox"
                                                                class="checkbox checkbox-xs"
                                                                prop:checked=has_checklist
                                                                on:change=make_toggle(FEATURE_CHECKLIST)
                                                            />
                                                            {move_tr!("lists-feature-checklist")}
                                                        </label>
                                                    </li>
                                                    <li>
                                                        <label class="flex items-center gap-2 cursor-pointer">
                                                            <input
                                                                type="checkbox"
                                                                class="checkbox checkbox-xs"
                                                                prop:checked=has_time_tracking
                                                                on:change=make_toggle(FEATURE_TIME_TRACKING)
                                                            />
                                                            {move_tr!("lists-feature-time-tracking")}
                                                        </label>
                                                    </li>
                                                }
                                            }
                                            <li>
                                                <button
                                                    type="button"
                                                    class="text-warning"
                                                    data-testid="action-archive"
                                                    on:click=move |_| {
                                                        confirm_list_name.set(list_name_for_archive.clone());
                                                        confirm_action.set(Some(ConfirmAction::Archive));
                                                    }
                                                >
                                                    {if list_archived { "📂 Przywróć" } else { "🗄 Archiwizuj" }}
                                                </button>
                                            </li>
                                            <li>
                                                <button
                                                    type="button"
                                                    class="text-error"
                                                    data-testid="action-delete"
                                                    on:click=move |_| {
                                                        confirm_list_name.set(list_name_for_delete.clone());
                                                        confirm_action.set(Some(ConfirmAction::Delete));
                                                    }
                                                >
                                                    "🗑 Usuń listę"
                                                </button>
                                            </li>
                                        </ul>
                                    </div>
                                </div>
                                <p class="text-xs text-base-content/40 mb-4" data-testid="list-created-at">
                                    "Utworzono: " {created_at_local}
                                </p>

                                <div class="mb-4">
                                    <EditableText
                                        value=list_description.clone().unwrap_or_default()
                                        on_save=Callback::new(move |new_desc: String| {
                                            let lid = list_id();
                                            let current_name = list_name_for_desc.clone();
                                            let val = if new_desc.trim().is_empty() { None } else { Some(new_desc) };
                                            leptos::task::spawn_local(async move {
                                                match rename_list(lid, current_name, val).await {
                                                    Ok(_) => set_refresh.update(|n| *n += 1),
                                                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                                }
                                            });
                                        })
                                        multiline=true
                                        placeholder="Dodaj opis..."
                                        class="text-base-content/60 cursor-pointer hover:underline decoration-dotted"
                                    />
                                </div>

                                // Tags
                                <div class="mb-4" data-testid="list-tags-section">
                                    {move || tag_res.get().and_then(|r| r.ok()).map(|(all_tags, tag_ids)| {
                                        view! {
                                            <TagList
                                                all_tags=all_tags
                                                selected_tag_ids=tag_ids
                                                on_toggle=on_tag_toggle
                                            />
                                        }
                                    })}
                                </div>

                                // Tag filter bar (shown only when items have tags)
                                <TagFilterBar
                                    all_tags=all_tags_for_filter
                                    item_tag_links=item_tag_links
                                    active_tag=active_tag
                                />

                                // Add item input
                                <div class="mb-4">
                                    <AddItemInput
                                        list_id=Signal::derive(list_id)
                                        has_quantity=has_quantity
                                        on_created=Callback::new(move |_| set_refresh.update(|n| *n += 1))
                                    />
                                </div>

                                // Sublist detach zone — visible when dragging a sublist of this list.
                                {
                                    let sublist_ids_for_detach = sublist_ids.clone();
                                    let detach_visible = Signal::derive(move || dnd_state.with(|s| {
                                        s.dragged
                                            .as_ref()
                                            .map(|d| d.kind == EntityKind::List && sublist_ids_for_detach.iter().any(|x| x == &d.id))
                                            .unwrap_or(false)
                                    }));
                                    let parent_container_for_detach = parent_container_id.clone();
                                    let on_sublist_detach = Callback::new(move |_| {
                                        let Some(id) = dnd_state.with_untracked(|s| s.dragged.as_ref().map(|d| d.id.clone())) else { return };
                                        let ctr = parent_container_for_detach.clone();
                                        leptos::task::spawn_local(async move {
                                            match move_list(id, ctr, None).await {
                                                Ok(_) => {
                                                    set_refresh.update(|n| *n += 1);
                                                    global_refresh.bump();
                                                }
                                                Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                            }
                                        });
                                    });
                                    view! {
                                        <DetachDropZone
                                            dnd_state=dnd_state
                                            visible=detach_visible
                                            on_drop=on_sublist_detach
                                            label="Upuść tutaj, aby odpiąć podlistę"
                                        />
                                    }
                                }

                                // Sublists section
                                {
                                    let notify = Callback::new(move |_| set_refresh.update(|n| *n += 1));
                                    let targets_for_subs = move_targets_all.clone();
                                    let sub_ids_for_subs = sublist_ids.clone();
                                    view! {
                                        <div class="mb-4">
                                            {if !sublists.is_empty() {
                                                let targets = targets_for_subs.clone();
                                                let sub_ids = sub_ids_for_subs.clone();
                                                view! {
                                                    <h3 class="text-sm font-semibold text-base-content/60 mb-2 uppercase tracking-wide">
                                                        "Podlisty"
                                                    </h3>
                                                    <div class="flex flex-col gap-2 mb-2">
                                                        {sublists.into_iter().map(|sublist| {
                                                            let sid = sublist.id.clone();
                                                            let mt: Vec<(String, String)> = targets
                                                                .iter()
                                                                .filter(|(tid, _)| tid != &sid && !sub_ids.iter().any(|s| s == tid && s == &sid))
                                                                .cloned()
                                                                .collect();
                                                            view! {
                                                                <SublistSection
                                                                    sublist=sublist
                                                                    on_any_change=notify
                                                                    move_targets=mt
                                                                    dnd_state=dnd_state
                                                                    item_dnd_state=item_dnd_state
                                                                    on_item_drop=on_item_drop
                                                                />
                                                            }
                                                        }).collect::<Vec<_>>()}
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! {}.into_any()
                                            }}
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
                                                    />
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <button
                                                        type="button"
                                                        class="btn btn-ghost btn-sm"
                                                        on:click=move |_| set_adding_sublist.set(true)
                                                    >
                                                        "+ Dodaj podlistę"
                                                    </button>
                                                }.into_any()
                                            }}
                                        </div>
                                    }.into_any()
                                }

                                // Items list
                                {move || {
                                    let tag_filter = active_tag.get();
                                    let visible: Vec<_> = all_items
                                        .iter()
                                        .filter(|i| show_completed.get() || !i.completed)
                                        .filter(|i| {
                                            tag_filter.as_ref().map(|tid| {
                                                item_tag_links_filter
                                                    .iter()
                                                    .any(|l| l.item_id == i.id && l.tag_id.as_str() == tid.as_str())
                                            }).unwrap_or(true)
                                        })
                                        .cloned()
                                        .collect();
                                    if visible.is_empty() {
                                        view! {
                                            <div class="text-center text-base-content/50 py-8">
                                                {if tag_filter.is_some() {
                                                    "Brak elementów z tym tagiem."
                                                } else if !show_completed.get() && completed_count.get() > 0 {
                                                    "Wszystkie elementy ukończone — odznacz filtr aby je zobaczyć."
                                                } else {
                                                    "Brak elementów. Dodaj pierwszy powyżej."
                                                }}
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div>
                                                <div class="flex items-center justify-between mb-2">
                                                    <span class="text-sm text-base-content/60" data-testid="completion-count">
                                                        {move || completed_count.get()} "/" {move || total.get()} " ukończone"
                                                    </span>
                                                    <label class="flex items-center gap-2 cursor-pointer select-none">
                                                        <span class="text-xs text-base-content/50">"Ukryj ukończone"</span>
                                                        <input
                                                            type="checkbox"
                                                            class="toggle toggle-xs"
                                                            data-testid="hide-completed-toggle"
                                                            prop:checked=move || !show_completed.get()
                                                            on:change=move |ev| set_show_completed.set(!event_target_checked(&ev))
                                                        />
                                                    </label>
                                                </div>
                                                {if has_deadlines {
                                                    let (overdue, upcoming, done) = partition_by_deadline(&visible, &today_date);
                                                    let render_section = |items: Vec<kartoteka_shared::types::Item>, label: &'static str, label_class: &'static str| {
                                                        if items.is_empty() { return view! {}.into_any(); }
                                                        let rows = items.into_iter().map(|item| {
                                                            let item_tags: Vec<kartoteka_shared::types::Tag> = item_tag_links_filter
                                                                .iter()
                                                                .filter(|l| l.item_id == item.id)
                                                                .filter_map(|l| all_tags_for_rows.iter().find(|t| t.id == l.tag_id).cloned())
                                                                .collect();
                                                            let mt = targets_for_items.clone();
                                                            let iid_tag = item.id.clone();
                                                            let selector_tags = all_tags_for_selector.clone();
                                                            let tag_cb = Callback::new(move |tag_id: String| {
                                                                on_item_tag_toggle.run((iid_tag.clone(), tag_id));
                                                            });
                                                            view! {
                                                                <ItemRow
                                                                    item=item
                                                                    item_tags=item_tags
                                                                    on_toggle=on_toggle_item
                                                                    on_delete=on_delete_item
                                                                    has_quantity=has_quantity
                                                                    list_features=current_features.clone()
                                                                    on_quantity_change=on_quantity_change
                                                                    on_description_save=on_description_save
                                                                    on_date_save=on_date_save
                                                                    move_targets=mt
                                                                    on_move=on_move_item
                                                                    on_tag_toggle=tag_cb
                                                                    all_tags_for_selector=selector_tags
                                                                />
                                                            }
                                                        }).collect::<Vec<_>>();
                                                        view! {
                                                            <div class="mb-4">
                                                                <p class=format!("text-xs font-semibold uppercase tracking-wide mb-2 {label_class}")>{label}</p>
                                                                <div class="flex flex-col gap-2">{rows}</div>
                                                            </div>
                                                        }.into_any()
                                                    };
                                                    view! {
                                                        <div class="flex flex-col">
                                                            {render_section(overdue, "Zaległe", "text-error")}
                                                            {render_section(upcoming, "Nadchodzące", "text-base-content/60")}
                                                            {render_section(done, "Ukończone", "text-base-content/40")}
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div class="flex flex-col">
                                                            {visible.into_iter().map(|item| {
                                                                let item_tags: Vec<kartoteka_shared::types::Tag> = item_tag_links_filter
                                                                    .iter()
                                                                    .filter(|l| l.item_id == item.id)
                                                                    .filter_map(|l| all_tags_for_rows.iter().find(|t| t.id == l.tag_id).cloned())
                                                                    .collect();
                                                                let mt = targets_for_items.clone();
                                                                let iid = item.id.clone();
                                                                let iid_tag = item.id.clone();
                                                                let before_tgt = ItemDropTarget::before(current_list_id.clone(), iid);
                                                                let selector_tags = all_tags_for_selector.clone();
                                                                let tag_cb = Callback::new(move |tag_id: String| {
                                                                    on_item_tag_toggle.run((iid_tag.clone(), tag_id));
                                                                });
                                                                view! {
                                                                    <ItemDropTargetMarker
                                                                        dnd_state=item_dnd_state
                                                                        target=before_tgt
                                                                        on_drop=on_item_drop
                                                                    />
                                                                    <ItemRow
                                                                        item=item
                                                                        item_tags=item_tags
                                                                        on_toggle=on_toggle_item
                                                                        on_delete=on_delete_item
                                                                        has_quantity=has_quantity
                                                                        list_features=current_features.clone()
                                                                        on_quantity_change=on_quantity_change
                                                                        on_description_save=on_description_save
                                                                        on_date_save=on_date_save
                                                                        move_targets=mt
                                                                        on_move=on_move_item
                                                                        dnd_state=item_dnd_state
                                                                        on_tag_toggle=tag_cb
                                                                        all_tags_for_selector=selector_tags
                                                                    />
                                                                }
                                                            }).collect::<Vec<_>>()}
                                                            <ItemDropTargetMarker
                                                                dnd_state=item_dnd_state
                                                                target=ItemDropTarget::end(current_list_id.clone())
                                                                on_drop=on_item_drop
                                                                label="Upuść na koniec"
                                                            />
                                                        </div>
                                                    }.into_any()
                                                }}
                                            </div>
                                        }.into_any()
                                    }
                                }}

                                // Comments
                                <CommentSection
                                    entity_type="list"
                                    entity_id=Signal::derive(list_id)
                                />
                            </div>
                        }.into_any()
                    }
                })}
            </Transition>

            {move || {
                let action = confirm_action.get()?;
                let name = confirm_list_name.get();
                let (title, message, label, variant) = match action {
                    ConfirmAction::Delete => (
                        "Usuń listę",
                        format!("Czy na pewno chcesz usunąć listę \"{}\"? Tej operacji nie można cofnąć.", name),
                        "Usuń",
                        ConfirmVariant::Danger,
                    ),
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
                let close = Callback::new(move |_| confirm_action.set(None));
                Some(view! {
                    <ConfirmModal
                        open=Signal::derive(move || confirm_action.get().is_some())
                        title=title.to_string()
                        message=message
                        confirm_label=label.to_string()
                        variant=variant
                        on_close=close
                        on_confirm=Callback::new(move |_| {
                            confirm_action.set(None);
                            let lid = list_id();
                            let nav2 = nav.clone();
                            match action {
                                ConfirmAction::Delete => leptos::task::spawn_local(async move {
                                    match delete_list(lid).await {
                                        Ok(_) => nav2("/", Default::default()),
                                        Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                    }
                                }),
                                ConfirmAction::Archive => leptos::task::spawn_local(async move {
                                    match archive_list(lid).await {
                                        Ok(_) => nav2("/", Default::default()),
                                        Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                    }
                                }),
                                ConfirmAction::Reset => leptos::task::spawn_local(async move {
                                    match reset_list(lid).await {
                                        Ok(_) => set_refresh.update(|n| *n += 1),
                                        Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                    }
                                }),
                            }
                        })
                    />
                })
            }}
        </div>
    }
}

/// Split items into (overdue, upcoming, done) buckets for the deadline date-view.
/// - done: `item.completed`
/// - overdue: not completed and deadline < today
/// - upcoming: everything else (no deadline, deadline >= today)
fn partition_by_deadline(
    items: &[kartoteka_shared::types::Item],
    today: &str,
) -> (
    Vec<kartoteka_shared::types::Item>,
    Vec<kartoteka_shared::types::Item>,
    Vec<kartoteka_shared::types::Item>,
) {
    let mut overdue = Vec::new();
    let mut upcoming = Vec::new();
    let mut done = Vec::new();

    for item in items {
        if item.completed {
            done.push(item.clone());
        } else if item
            .deadline
            .as_ref()
            .map(|d| d.start().format("%Y-%m-%d").to_string().as_str() < today)
            .unwrap_or(false)
        {
            overdue.push(item.clone());
        } else {
            upcoming.push(item.clone());
        }
    }

    (overdue, upcoming, done)
}
