//! Aggregated (recursive) tracked-time totals for items, lists and containers.
//!
//! A total is every completed time entry in scope plus the live elapsed time of a
//! running entry in scope. Lists include all their sublists at any depth, and
//! containers include all sub-containers, their lists and those lists' sublists.
//! Inbox entries (no item) never count towards an item, list or container.

use kartoteka_shared::types::TimeTotal;
use leptos::prelude::*;
use std::collections::HashMap;

#[cfg(feature = "ssr")]
use {
    crate::server_fns::utils::{SfnResultExt, require_user},
    kartoteka_domain as domain,
};

#[cfg(feature = "ssr")]
fn domain_total_to_shared(t: domain::time_entries::TimeTotal) -> TimeTotal {
    TimeTotal {
        seconds: t.seconds,
        running_since: t.running_since,
    }
}

/// Tracked time for one item (completed entries + live running entry).
#[server(prefix = "/leptos")]
pub async fn get_item_time_total(item_id: String) -> Result<TimeTotal, ServerFnError> {
    let (pool, user) = require_user().await?;
    let total = domain::time_entries::total_for_item(&pool, &user.id, &item_id)
        .await
        .sfn()?;
    Ok(domain_total_to_shared(total))
}

/// Tracked time for a list **including every sublist at any depth**.
#[server(prefix = "/leptos")]
pub async fn get_list_time_total(list_id: String) -> Result<TimeTotal, ServerFnError> {
    let (pool, user) = require_user().await?;
    let total = domain::time_entries::total_for_list(&pool, &user.id, &list_id)
        .await
        .sfn()?;
    Ok(domain_total_to_shared(total))
}

/// Tracked time for a container **including all sub-containers, their lists and
/// those lists' sublists**.
#[server(prefix = "/leptos")]
pub async fn get_container_time_total(container_id: String) -> Result<TimeTotal, ServerFnError> {
    let (pool, user) = require_user().await?;
    let total = domain::time_entries::total_for_container(&pool, &user.id, &container_id)
        .await
        .sfn()?;
    Ok(domain_total_to_shared(total))
}

/// Per-item totals for list pages — one query, no N+1. Every requested id is
/// present in the map (`0:00` when it has no entries).
#[server(prefix = "/leptos")]
pub async fn get_items_time_totals(
    item_ids: Vec<String>,
) -> Result<HashMap<String, TimeTotal>, ServerFnError> {
    let (pool, user) = require_user().await?;
    let totals = domain::time_entries::totals_for_items(&pool, &user.id, &item_ids)
        .await
        .sfn()?;
    Ok(totals
        .into_iter()
        .map(|(id, t)| (id, domain_total_to_shared(t)))
        .collect())
}

/// Per-list totals (each rolling up its own sublists) — one query, no N+1.
#[server(prefix = "/leptos")]
pub async fn get_lists_time_totals(
    list_ids: Vec<String>,
) -> Result<HashMap<String, TimeTotal>, ServerFnError> {
    let (pool, user) = require_user().await?;
    let totals = domain::time_entries::totals_for_lists(&pool, &user.id, &list_ids)
        .await
        .sfn()?;
    Ok(totals
        .into_iter()
        .map(|(id, t)| (id, domain_total_to_shared(t)))
        .collect())
}
