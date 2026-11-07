//! Server functions behind `/trash`, the review page for the pending-deletions queue
//! (#226).
//!
//! Marking is the only thing an agent can do; the user decides here what actually goes.
//! Deleting from the queue cascades through items, which is why every call that deletes
//! takes the exact keys the user confirmed and nothing more.
//!
//! The functions that carry keys use the JSON input codec: `Vec<PendingDeletionKey>` has
//! a nested shape, and `toggle_pending_deletion` is also called directly by the e2e suite,
//! so its `endpoint` is pinned.

use kartoteka_shared::{DeletableEntity, PendingDeletionItem, PendingDeletionKey};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    crate::server_fns::utils::{SfnResultExt, require_user},
    kartoteka_domain as domain,
};

/// The user's queue, resolved for display and ordered newest first.
#[server(prefix = "/leptos")]
pub async fn get_pending_deletions() -> Result<Vec<PendingDeletionItem>, ServerFnError> {
    let (pool, user) = require_user().await?;
    domain::pending_deletions::list_all(&pool, &user.id)
        .await
        .sfn()
}

/// Number of rows in the queue, for the navbar badge.
///
/// Counts the queue rows themselves: the migration-021 triggers remove a row as soon as
/// its entity disappears, so this agrees with [`get_pending_deletions`] without resolving
/// every row's label, context and link.
#[server(prefix = "/leptos")]
pub async fn get_pending_deletion_count() -> Result<u32, ServerFnError> {
    let (pool, user) = require_user().await?;
    let rows = kartoteka_db::pending_deletions::list_for_user(&pool, &user.id)
        .await
        .sfn()?;
    Ok(u32::try_from(rows.len()).unwrap_or(u32::MAX))
}

/// Mark or unmark one entity. Returns `true` when it is now marked.
///
/// The endpoint is pinned: the e2e suite marks entities by POSTing JSON straight at
/// `/leptos/toggle_pending_deletion` instead of hunting for the button on every page.
#[server(
    prefix = "/leptos",
    endpoint = "toggle_pending_deletion",
    input = ::leptos::server_fn::codec::Json
)]
pub async fn toggle_pending_deletion(
    entity_type: DeletableEntity,
    entity_id: String,
) -> Result<bool, ServerFnError> {
    let (pool, user) = require_user().await?;
    domain::pending_deletions::toggle(&pool, &user.id, entity_type, &entity_id)
        .await
        .sfn()
}

/// Remove rows from the queue without deleting the entities. Returns how many rows went.
#[server(prefix = "/leptos", input = ::leptos::server_fn::codec::Json)]
pub async fn unmark_pending_deletions(keys: Vec<PendingDeletionKey>) -> Result<u32, ServerFnError> {
    let (pool, user) = require_user().await?;
    domain::pending_deletions::unmark(&pool, &user.id, &keys)
        .await
        .sfn()
}

/// Hard-delete the given entities, cascading through their subtree and items.
/// Returns how many of the keys were actually deleted.
#[server(prefix = "/leptos", input = ::leptos::server_fn::codec::Json)]
pub async fn delete_pending_deletions(keys: Vec<PendingDeletionKey>) -> Result<u32, ServerFnError> {
    let (pool, user) = require_user().await?;
    domain::pending_deletions::delete(&pool, &user.id, &keys)
        .await
        .sfn()
}

/// Hard-delete everything in the queue. Returns how many entities were deleted.
#[server(prefix = "/leptos")]
pub async fn delete_all_pending_deletions() -> Result<u32, ServerFnError> {
    let (pool, user) = require_user().await?;
    domain::pending_deletions::delete_all(&pool, &user.id)
        .await
        .sfn()
}
