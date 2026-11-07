//! `/trash` — the review page for entities marked for deletion (#226, #389).
//!
//! Agents and (later) MCP tools can only *mark*; nothing is deleted until the user
//! decides here. Deleting from this queue is the one path in the app that cascades
//! through items (the #223 rule "items always block" does not apply), so every delete
//! goes through a confirmation that spells out the subtree it takes along.
//!
//! Unmarking is harmless — it only removes the queue row — and needs no confirmation.

use std::collections::{HashMap, HashSet};

use kartoteka_shared::{ChangeTopic, DeletableEntity, PendingDeletionItem, PendingDeletionKey};
use leptos::prelude::*;
use leptos_fluent::{I18n, move_tr};

use crate::components::common::confirm_modal::{ConfirmModal, ConfirmVariant};
use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::pending_deletions::{
    delete_pending_deletions, get_pending_deletions, unmark_pending_deletions,
};
use crate::state::invalidation::{mutate_with, use_invalidation};
use crate::utils::{AgeBucket, age_bucket, now_millis};

/// Topics whose data can change the queue: a row disappears when its entity does,
/// whether by a single delete, a cascade or the migration-021 orphan cleanup.
const QUEUE_TOPICS: [ChangeTopic; 4] = [
    ChangeTopic::Items,
    ChangeTopic::Lists,
    ChangeTopic::Containers,
    ChangeTopic::Comments,
];

/// The page sections, in display order.
const SECTIONS: [DeletableEntity; 4] = [
    DeletableEntity::Container,
    DeletableEntity::List,
    DeletableEntity::Item,
    DeletableEntity::Comment,
];

// ── small helpers ─────────────────────────────────────────────────────────────

fn key_of(item: &PendingDeletionItem) -> PendingDeletionKey {
    PendingDeletionKey {
        entity_type: item.entity_type,
        entity_id: item.entity_id.clone(),
    }
}

fn as_i64(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

fn tr_count(i18n: &I18n, key: &str, count: i64) -> String {
    let mut args = HashMap::new();
    args.insert("count".into(), count.into());
    i18n.tr_with_args(key, &args)
}

fn tr_details(i18n: &I18n, key: &str, details: String) -> String {
    let mut args = HashMap::new();
    args.insert("details".into(), details.into());
    i18n.tr_with_args(key, &args)
}

fn type_icon(kind: DeletableEntity) -> &'static str {
    match kind {
        DeletableEntity::Container => "📁",
        DeletableEntity::List => "📋",
        DeletableEntity::Item => "☑",
        DeletableEntity::Comment => "💬",
    }
}

/// "3 sublists, 12 items": what a cascade takes along, or `None` when it takes nothing.
fn cascade_details(i18n: &I18n, lists: i64, containers: i64, items: i64) -> Option<String> {
    let mut parts = Vec::new();
    if lists > 0 {
        parts.push(tr_count(i18n, "trash-cascade-lists", lists));
    }
    if containers > 0 {
        parts.push(tr_count(i18n, "trash-cascade-containers", containers));
    }
    if items > 0 {
        parts.push(tr_count(i18n, "trash-cascade-items", items));
    }
    (!parts.is_empty()).then(|| parts.join(", "))
}

// ── the delete the user asked for ─────────────────────────────────────────────

/// A delete waiting for confirmation, with the numbers the dialog has to show.
#[derive(Clone, PartialEq, Eq)]
struct DeleteRequest {
    /// Queue keys to delete: exactly the rows the user saw.
    keys: Vec<PendingDeletionKey>,
    /// How many queue rows go away.
    count: usize,
    /// Summed `contains` of the selected rows, i.e. what the cascade takes along.
    lists: i64,
    containers: i64,
    items: i64,
}

/// Build the confirmation payload for the given rows.
fn request_for(rows: &[PendingDeletionItem]) -> DeleteRequest {
    let (lists, containers, items) = rows.iter().fold((0, 0, 0), |acc, row| match &row.contains {
        Some(preview) => (
            acc.0 + preview.lists,
            acc.1 + preview.containers,
            acc.2 + preview.items,
        ),
        None => acc,
    });
    DeleteRequest {
        keys: rows.iter().map(key_of).collect(),
        count: rows.len(),
        lists,
        containers,
        items,
    }
}

/// The confirmation sentence: how many rows go, and — when the cascade reaches further —
/// what else disappears with them.
fn delete_message(i18n: &I18n, request: &DeleteRequest) -> String {
    let mut message = tr_count(i18n, "trash-confirm-delete-message", as_i64(request.count));
    if let Some(details) = cascade_details(i18n, request.lists, request.containers, request.items) {
        message.push(' ');
        message.push_str(&tr_details(i18n, "trash-confirm-delete-cascade", details));
    }
    message
}

// ── one row ───────────────────────────────────────────────────────────────────

#[component]
fn TrashRow(
    item: PendingDeletionItem,
    selection: RwSignal<HashSet<PendingDeletionKey>>,
    /// Unmark one row (no confirmation: nothing is deleted).
    on_unmark: Callback<PendingDeletionKey>,
    /// Ask to delete this row; the page owns the confirmation dialog.
    on_delete: Callback<PendingDeletionItem>,
) -> impl IntoView {
    let i18n = expect_context::<I18n>();

    let kind = item.entity_type;
    let icon = type_icon(kind);
    let check_key = key_of(&item);
    let change_key = check_key.clone();
    let unmark_key = check_key.clone();
    let delete_item = item.clone();

    let age = match age_bucket(&item.marked_at, now_millis()) {
        Some(AgeBucket::JustNow) => move_tr!("trash-marked-just-now"),
        Some(AgeBucket::Minutes(count)) => move_tr!("trash-marked-minutes", { "count" => count }),
        Some(AgeBucket::Hours(count)) => move_tr!("trash-marked-hours", { "count" => count }),
        Some(AgeBucket::Days(count)) => move_tr!("trash-marked-days", { "count" => count }),
        None => move_tr!("trash-marked-unknown"),
    };

    let cascade = item
        .contains
        .and_then(|preview| {
            cascade_details(&i18n, preview.lists, preview.containers, preview.items)
        })
        .map(|details| tr_details(&i18n, "trash-cascade-badge", details));

    let label = item.label.clone();
    let label_view = match item.href.clone() {
        Some(href) => view! {
            <a
                href=href
                class="font-medium link link-hover break-words"
                data-testid="trash-row-label"
            >
                {label}
            </a>
        }
        .into_any(),
        None => view! {
            <span class="font-medium break-words" data-testid="trash-row-label">{label}</span>
        }
        .into_any(),
    };

    view! {
        <div
            class="flex items-start gap-3 rounded-box border border-base-300 bg-base-100 p-3"
            data-testid="trash-row"
            data-entity-type=kind.as_str()
        >
            <input
                type="checkbox"
                class="checkbox checkbox-sm mt-1"
                data-testid="trash-row-checkbox"
                prop:checked=move || selection.with(|chosen| chosen.contains(&check_key))
                on:change=move |_| {
                    let marked = selection.with_untracked(|chosen| chosen.contains(&change_key));
                    selection.update(|chosen| {
                        if marked {
                            chosen.remove(&change_key);
                        } else {
                            chosen.insert(change_key.clone());
                        }
                    });
                }
            />
            <span class="text-xl leading-none" aria-hidden="true">{icon}</span>
            <div class="min-w-0 flex-1">
                <div class="flex flex-wrap items-center gap-2">
                    {label_view}
                    {item
                        .context
                        .clone()
                        .map(|context| {
                            view! {
                                <span
                                    class="text-xs text-base-content/60"
                                    data-testid="trash-row-context"
                                >
                                    {context}
                                </span>
                            }
                        })}
                    {cascade
                        .map(|badge| {
                            view! {
                                <span
                                    class="badge badge-warning badge-sm"
                                    data-testid="trash-row-cascade"
                                >
                                    {badge}
                                </span>
                            }
                        })}
                </div>
                <div class="text-xs text-base-content/50 mt-1" data-testid="trash-row-marked-at">
                    {age}
                </div>
            </div>
            <div class="flex shrink-0 gap-1">
                <button
                    type="button"
                    class="btn btn-ghost btn-xs"
                    data-testid="trash-row-unmark"
                    on:click=move |_| on_unmark.run(unmark_key.clone())
                >
                    {move_tr!("trash-action-unmark")}
                </button>
                <button
                    type="button"
                    class="btn btn-error btn-xs"
                    data-testid="trash-row-delete"
                    on:click=move |_| on_delete.run(delete_item.clone())
                >
                    {move_tr!("trash-action-delete")}
                </button>
            </div>
        </div>
    }
}

// ── page ──────────────────────────────────────────────────────────────────────

#[component]
pub fn TrashPage() -> impl IntoView {
    let i18n = expect_context::<I18n>();
    let invalidation = use_invalidation();

    let items_res = Resource::new(
        move || invalidation.track(&QUEUE_TOPICS),
        |_| get_pending_deletions(),
    );

    let selection: RwSignal<HashSet<PendingDeletionKey>> = RwSignal::new(HashSet::new());
    let request: RwSignal<Option<DeleteRequest>> = RwSignal::new(None);

    let on_unmark_one = Callback::new(move |key: PendingDeletionKey| {
        selection.update(|chosen| {
            chosen.remove(&key);
        });
        mutate_with(
            unmark_pending_deletions(vec![key]),
            &QUEUE_TOPICS,
            |_removed| {},
        );
    });

    let on_delete_one = Callback::new(move |item: PendingDeletionItem| {
        request.set(Some(request_for(std::slice::from_ref(&item))));
    });

    let on_confirm = Callback::new(move |_: ()| {
        let Some(pending) = request.get_untracked() else {
            return;
        };
        request.set(None);
        // Always the exact keys shown in the confirmation, also for "empty trash": rows
        // marked after the page loaded (e.g. by an agent) must not go along unseen.
        let keys = pending.keys.clone();
        mutate_with(
            async move { delete_pending_deletions(keys).await },
            &QUEUE_TOPICS,
            move |_deleted| selection.set(HashSet::new()),
        );
    });

    let on_close = Callback::new(move |_: ()| request.set(None));

    view! {
        <div class="container mx-auto max-w-3xl p-4" data-testid="trash-page">
            <h1 class="text-2xl font-bold mb-1">{move_tr!("trash-title")}</h1>
            <p class="text-sm text-base-content/60 mb-6">{move_tr!("trash-subtitle")}</p>

            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || items_res.get().map(|result| match result {
                    Err(e) => {
                        let detail = e.to_string();
                        view! {
                            <p class="text-error">
                                {move_tr!("trash-loading-error", { "detail" => detail.clone() })}
                            </p>
                        }
                            .into_any()
                    }
                    Ok(items) => {
                        if items.is_empty() {
                            return view! {
                                <div
                                    class="text-center text-base-content/50 py-12"
                                    data-testid="trash-empty"
                                >
                                    {move_tr!("trash-empty")}
                                </div>
                            }
                                .into_any();
                        }

                        let all_keys: Vec<PendingDeletionKey> = items.iter().map(key_of).collect();
                        let keys_for_all = StoredValue::new(all_keys);
                        let items_for_all = StoredValue::new(items.clone());
                        let items_for_selected = StoredValue::new(items.clone());

                        let on_select_all = move |_| {
                            let keys = keys_for_all.get_value();
                            let every_selected =
                                selection.with(|chosen| keys.iter().all(|key| chosen.contains(key)));
                            selection.set(if every_selected {
                                HashSet::new()
                            } else {
                                keys.into_iter().collect()
                            });
                        };

                        let on_unmark_selected = move |_| {
                            let keys: Vec<PendingDeletionKey> =
                                selection.get_untracked().into_iter().collect();
                            if keys.is_empty() {
                                return;
                            }
                            mutate_with(
                                unmark_pending_deletions(keys),
                                &QUEUE_TOPICS,
                                move |_removed| selection.set(HashSet::new()),
                            );
                        };

                        let on_delete_selected = move |_| {
                            let chosen = selection.get_untracked();
                            let rows: Vec<PendingDeletionItem> = items_for_selected
                                .get_value()
                                .into_iter()
                                .filter(|item| chosen.contains(&key_of(item)))
                                .collect();
                            if rows.is_empty() {
                                return;
                            }
                            request.set(Some(request_for(&rows)));
                        };

                        let on_delete_all = move |_| {
                            let rows = items_for_all.get_value();
                            request.set(Some(request_for(&rows)));
                        };

                        view! {
                            <div>
                                <div
                                    class="flex flex-wrap items-center gap-2 mb-4 rounded-box border border-base-300 bg-base-200 p-2"
                                    data-testid="trash-bulk-bar"
                                >
                                    <label class="flex items-center gap-2 text-sm cursor-pointer select-none">
                                        <input
                                            type="checkbox"
                                            class="checkbox checkbox-sm"
                                            data-testid="trash-select-all"
                                            prop:checked=move || {
                                                let keys = keys_for_all.get_value();
                                                !keys.is_empty()
                                                    && selection
                                                        .with(|chosen| {
                                                            keys.iter().all(|key| chosen.contains(key))
                                                        })
                                            }
                                            on:change=on_select_all
                                        />
                                        {move_tr!("trash-select-all")}
                                    </label>
                                    <span
                                        class="text-xs text-base-content/60"
                                        data-testid="trash-selected-count"
                                    >
                                        {move_tr!(
                                            "trash-selected",
                                            { "count" => as_i64(selection.with(HashSet::len)) },
                                        )}
                                    </span>
                                    <button
                                        type="button"
                                        class="btn btn-ghost btn-sm"
                                        data-testid="trash-bulk-unmark"
                                        prop:disabled=move || selection.with(HashSet::is_empty)
                                        on:click=on_unmark_selected
                                    >
                                        {move_tr!("trash-action-unmark-selected")}
                                    </button>
                                    <button
                                        type="button"
                                        class="btn btn-error btn-sm"
                                        data-testid="trash-bulk-delete"
                                        prop:disabled=move || selection.with(HashSet::is_empty)
                                        on:click=on_delete_selected
                                    >
                                        {move_tr!("trash-action-delete-selected")}
                                    </button>
                                    <button
                                        type="button"
                                        class="btn btn-error btn-outline btn-sm ml-auto"
                                        data-testid="trash-bulk-delete-all"
                                        on:click=on_delete_all
                                    >
                                        {move_tr!("trash-action-delete-all")}
                                    </button>
                                </div>

                                {SECTIONS
                                    .into_iter()
                                    .map(|kind| {
                                        let rows: Vec<PendingDeletionItem> = items
                                            .iter()
                                            .filter(|item| item.entity_type == kind)
                                            .cloned()
                                            .collect();
                                        if rows.is_empty() {
                                            return None;
                                        }
                                        let count = rows.len();
                                        let title = match kind {
                                            DeletableEntity::Container => {
                                                move_tr!("trash-section-containers")
                                            }
                                            DeletableEntity::List => move_tr!("trash-section-lists"),
                                            DeletableEntity::Item => move_tr!("trash-section-items"),
                                            DeletableEntity::Comment => {
                                                move_tr!("trash-section-comments")
                                            }
                                        };
                                        let icon = type_icon(kind);
                                        Some(view! {
                                            <section
                                                class="mb-6"
                                                data-testid="trash-section"
                                                data-entity-type=kind.as_str()
                                            >
                                                <h2 class="flex items-center gap-2 text-lg font-semibold mb-2">
                                                    <span aria-hidden="true">{icon}</span>
                                                    {title}
                                                    <span
                                                        class="text-sm font-normal text-base-content/50"
                                                        data-testid="trash-section-count"
                                                    >
                                                        {count}
                                                    </span>
                                                </h2>
                                                <div class="flex flex-col gap-2">
                                                    {rows
                                                        .into_iter()
                                                        .map(|item| {
                                                            view! {
                                                                <TrashRow
                                                                    item=item
                                                                    selection=selection
                                                                    on_unmark=on_unmark_one
                                                                    on_delete=on_delete_one
                                                                />
                                                            }
                                                        })
                                                        .collect::<Vec<_>>()}
                                                </div>
                                            </section>
                                        }
                                            .into_any())
                                    })
                                    .collect::<Vec<_>>()}
                            </div>
                        }
                            .into_any()
                    }
                })}
            </Suspense>

            {move || {
                let pending = request.get()?;
                Some(view! {
                    <ConfirmModal
                        open=Signal::derive(move || request.get().is_some())
                        title=i18n.tr("trash-confirm-delete-title")
                        message=delete_message(&i18n, &pending)
                        confirm_label=i18n.tr("trash-confirm-delete-button")
                        variant=ConfirmVariant::Danger
                        on_confirm=on_confirm
                        on_close=on_close
                    />
                })
            }}
        </div>
    }
}
