//! The list page header: breadcrumbs, icon and name, container controls, the actions menu,
//! creation date and description.
//!
//! It is created once per page and reads the list's metadata through memos of
//! [`ListPageState`], so a refetch touches only the fields that changed and the name and
//! description editors keep their state.

use leptos::prelude::*;
use leptos_fluent::move_tr;

use kartoteka_shared::{
    ChangeTopic, FEATURE_CHECKLIST, FEATURE_DEADLINES, FEATURE_LOCATION, FEATURE_QUANTITY,
    FEATURE_TIME_TRACKING, default_list_icon,
};

use super::state::ListPageState;
use crate::components::common::breadcrumbs::Breadcrumbs;
use crate::components::common::container_selector_dropdown::ContainerSelectorDropdown;
use crate::components::common::delete_entity_modal::DeleteTarget;
use crate::components::common::editable_text::EditableText;
use crate::components::lists::deadlines_config::DeadlinesConfig;
use crate::components::ui::{EntityIcon, Icon, IconButton, IconButtonSize, IconButtonVariant};
use crate::server_fns::containers::get_containers_for_move;
use crate::server_fns::lists::{move_list, pin_list, rename_list, update_list_features};
use crate::state::invalidation::{mutate, use_invalidation};

#[derive(Clone, Copy, PartialEq)]
pub enum ConfirmAction {
    Archive,
    Reset,
}

/// A confirmation the page shows in a modal: what to do, and the list's name for the text.
#[derive(Clone, Copy)]
pub struct ConfirmRequest {
    pub action: RwSignal<Option<ConfirmAction>>,
    pub list_name: RwSignal<String>,
}

#[component]
pub fn ListHeader(
    state: ListPageState,
    list_id: Signal<String>,
    confirm: ConfirmRequest,
    delete_target: RwSignal<Option<DeleteTarget>>,
) -> impl IntoView {
    let invalidation = use_invalidation();

    let name = state.field(|m| m.name.clone());
    let description = state.field(|m| m.description.clone().unwrap_or_default());
    let icon = state.field(|m| m.icon.clone());
    let list_type = state.field(|m| m.list_type.clone());
    let pinned = state.field(|m| m.pinned);
    let archived = state.field(|m| m.archived);
    let container_name = state.field(|m| m.container_name.clone());
    let created_at = state.field(|m| m.created_at_local.clone());
    let features = state.field(|m| m.features.clone());
    let crumbs = state.field(|m| {
        let mut crumbs = Vec::new();
        if let (Some(name), Some(id)) = (&m.container_name, &m.container_id) {
            crumbs.push((format!("/containers/{id}"), name.clone()));
        }
        if let Some(parent) = &m.parent_list_id {
            crumbs.push((
                format!("/lists/{parent}"),
                m.parent_list_name.clone().unwrap_or_default(),
            ));
        }
        crumbs
    });
    let has_feature = move |feature: &'static str| {
        Memo::new(move |_| features.with(|f| f.iter().any(|f| f == feature)))
    };
    let has_quantity = has_feature(FEATURE_QUANTITY);
    let has_deadlines = has_feature(FEATURE_DEADLINES);
    let has_location = has_feature(FEATURE_LOCATION);
    let has_checklist = has_feature(FEATURE_CHECKLIST);
    let has_time_tracking = has_feature(FEATURE_TIME_TRACKING);

    let container_dropdown_open = RwSignal::new(false);
    let containers_res = Resource::new(
        move || container_dropdown_open.get(),
        |open| async move {
            if open {
                get_containers_for_move(None, false).await
            } else {
                Ok(vec![])
            }
        },
    );
    let on_move_to_container = Callback::new(move |cid: String| {
        container_dropdown_open.set(false);
        mutate(
            move_list(list_id.get_untracked(), Some(cid), None),
            &[ChangeTopic::Lists, ChangeTopic::Containers],
        );
    });

    let make_toggle = move |feature: &'static str| {
        move |ev: leptos::ev::Event| {
            let checked = event_target_checked(&ev);
            let mut f = features.get_untracked();
            if checked {
                f.push(feature.to_string());
            } else {
                f.retain(|x| x != feature);
            }
            mutate(
                update_list_features(list_id.get_untracked(), f),
                &[ChangeTopic::Lists],
            );
        }
    };

    view! {
        {move || view! { <Breadcrumbs crumbs=crumbs.get() current=name.get() /> }}

        <div class="mb-1 flex items-center gap-2">
            {move || view! {
                <EntityIcon
                    raw=icon.get()
                    fallback=default_list_icon(&list_type.get())
                    size=24
                />
            }}
            <EditableText
                value=name
                on_save=Callback::new(move |new_name: String| {
                    mutate(
                        rename_list(list_id.get_untracked(), new_name, None),
                        &[ChangeTopic::Lists],
                    );
                })
                class="text-2xl font-bold cursor-pointer hover:underline decoration-dotted"
                testid="list-name-heading"
            />
            // Container move/detach
            {move || {
                let opts = containers_res.get().and_then(|r| r.ok()).unwrap_or_default();
                if let Some(cname) = container_name.get() {
                    view! {
                        <div class="flex items-center gap-1 shrink-0">
                            <span class="text-xs text-base-content/60 flex items-center gap-1">
                                <Icon name="folder" size=12 />
                                <span class="max-w-28 truncate">{cname}</span>
                            </span>
                            <IconButton
                                icon="x"
                                label=move_tr!("lists-detach-from-container")
                                variant=IconButtonVariant::Danger
                                on_click=Callback::new(move |_| {
                                    mutate(
                                        move_list(list_id.get_untracked(), None, None),
                                        &[ChangeTopic::Lists, ChangeTopic::Containers],
                                    );
                                })
                            />
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="relative shrink-0">
                            <IconButton
                                icon="folder"
                                label=move_tr!("lists-move-to-container")
                                on_click=Callback::new(move |_| container_dropdown_open.update(|v| *v = !*v))
                            />
                            <ContainerSelectorDropdown
                                open=container_dropdown_open
                                options=opts
                                on_select=on_move_to_container
                            />
                        </div>
                    }.into_any()
                }
            }}
            // Dropdown at the end, pushed right via ml-auto
            <div class="dropdown dropdown-end ml-auto">
                <IconButton
                    icon="more-vertical"
                    label=move_tr!("common-actions")
                    size=IconButtonSize::Sm
                    testid=Some("list-actions-btn")
                />
                <ul tabindex="0" class="dropdown-content menu bg-base-100 rounded-box z-50 w-52 p-2 shadow-lg border border-base-300">
                    <li>
                        <button
                            type="button"
                            data-testid="action-pin"
                            on:click=move |_| {
                                mutate(pin_list(list_id.get_untracked()), &[ChangeTopic::Lists]);
                            }
                        >
                            {move || view! { <Icon name=if pinned.get() { "pin-off" } else { "pin" } size=16 /> }}
                            {move || if pinned.get() { move_tr!("lists-unpin").get() } else { move_tr!("lists-pin").get() }}
                        </button>
                    </li>
                    <li>
                        <button
                            type="button"
                            data-testid="action-reset"
                            on:click=move |_| {
                                confirm.list_name.set(name.get_untracked());
                                confirm.action.set(Some(ConfirmAction::Reset));
                            }
                        >
                            <Icon name="timer" size=16 />
                            {move_tr!("lists-reset-completed")}
                        </button>
                    </li>
                    <li class="menu-title">
                        <span class="text-xs uppercase tracking-wide opacity-60">"Funkcje"</span>
                    </li>
                    <li>
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input
                                type="checkbox"
                                class="checkbox checkbox-xs"
                                prop:checked=move || has_quantity.get()
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
                                prop:checked=move || has_deadlines.get()
                                on:change=make_toggle(FEATURE_DEADLINES)
                            />
                            "Terminy"
                        </label>
                    </li>
                    // The config boxes keep their own state while the dropdown is open.
                    {move || has_deadlines.get().then(|| {
                        let config = state
                            .meta
                            .with_untracked(|m| m.as_ref().map(|m| m.deadlines_config.clone()))
                            .unwrap_or_default();
                        view! {
                            <DeadlinesConfig
                                list_id=list_id.get_untracked()
                                config=config
                                on_changed=Callback::new(move |_| invalidation.invalidate(ChangeTopic::Lists))
                            />
                        }
                    })}
                    <li>
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input
                                type="checkbox"
                                class="checkbox checkbox-xs"
                                prop:checked=move || has_location.get()
                                on:change=make_toggle(FEATURE_LOCATION)
                            />
                            <Icon name="pin" size=14 />
                            {move_tr!("lists-feature-location")}
                        </label>
                    </li>
                    <li>
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input
                                type="checkbox"
                                class="checkbox checkbox-xs"
                                prop:checked=move || has_checklist.get()
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
                                prop:checked=move || has_time_tracking.get()
                                on:change=make_toggle(FEATURE_TIME_TRACKING)
                            />
                            {move_tr!("lists-feature-time-tracking")}
                        </label>
                    </li>
                    <li>
                        <button
                            type="button"
                            class="text-warning"
                            data-testid="action-archive"
                            on:click=move |_| {
                                confirm.list_name.set(name.get_untracked());
                                confirm.action.set(Some(ConfirmAction::Archive));
                            }
                        >
                            <Icon name="archive" size=16 />
                            {move || if archived.get() { move_tr!("lists-restore-aria").get() } else { move_tr!("lists-header-archive-button").get() }}
                        </button>
                    </li>
                    <li>
                        <button
                            type="button"
                            class="text-error"
                            data-testid="action-delete"
                            on:click=move |_| {
                                let id = state
                                    .meta
                                    .with_untracked(|m| m.as_ref().map(|m| m.id.clone()))
                                    .unwrap_or_else(|| list_id.get_untracked());
                                delete_target.set(Some(DeleteTarget::list(id, name.get_untracked())));
                            }
                        >
                            <Icon name="trash" size=16 />
                            {move_tr!("lists-header-delete-button")}
                        </button>
                    </li>
                </ul>
            </div>
        </div>
        <p class="text-xs text-base-content/40 mb-4" data-testid="list-created-at">
            "Utworzono: " {created_at}
        </p>

        <div class="mb-4">
            <EditableText
                value=description
                on_save=Callback::new(move |new_desc: String| {
                    let val = if new_desc.trim().is_empty() { None } else { Some(new_desc) };
                    mutate(
                        rename_list(list_id.get_untracked(), name.get_untracked(), val),
                        &[ChangeTopic::Lists],
                    );
                })
                multiline=true
                placeholder="Dodaj opis..."
                class="text-base-content/60 cursor-pointer hover:underline decoration-dotted"
            />
        </div>
    }
}
