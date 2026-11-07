use kartoteka_shared::types::{ItemTimeSummary, TimeEntry};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    crate::server_fns::utils::{SfnResultExt, format_datetime_in_tz, require_user},
    kartoteka_domain as domain,
    sqlx::SqlitePool,
};

#[cfg(feature = "ssr")]
fn domain_entry_to_shared(e: domain::time_entries::TimeEntry) -> TimeEntry {
    TimeEntry {
        id: e.id,
        item_id: e.item_id,
        user_id: e.user_id,
        description: e.description,
        started_at: e.started_at,
        ended_at: e.ended_at,
        duration: e.duration,
        source: e.source,
        mode: e.mode,
        created_at: e.created_at,
    }
}

#[cfg(feature = "ssr")]
async fn user_timezone(pool: &SqlitePool, user_id: &str) -> Result<String, ServerFnError> {
    Ok(domain::preferences::get(pool, user_id)
        .await
        .sfn()?
        .timezone)
}

#[cfg(feature = "ssr")]
fn domain_summary_to_shared(s: domain::time_entries::ItemTimeSummary) -> ItemTimeSummary {
    ItemTimeSummary {
        total_seconds: s.total_seconds,
        entry_count: s.entry_count,
    }
}

#[server(prefix = "/leptos")]
pub async fn get_time_summary(item_id: String) -> Result<ItemTimeSummary, ServerFnError> {
    let (pool, user) = require_user().await?;
    let summary = domain::time_entries::summary_for_item(&pool, &user.id, &item_id)
        .await
        .sfn()?;
    Ok(domain_summary_to_shared(summary))
}

#[server(prefix = "/leptos")]
pub async fn get_running_timer() -> Result<Option<TimeEntry>, ServerFnError> {
    let (pool, user) = require_user().await?;
    let entry = kartoteka_db::time_entries::get_running(&pool, &user.id)
        .await
        .sfn()?;
    Ok(entry.map(|row| TimeEntry {
        id: row.id,
        item_id: row.item_id,
        user_id: row.user_id,
        description: row.description,
        started_at: row.started_at,
        ended_at: row.ended_at,
        duration: row.duration,
        source: row.source,
        mode: row.mode,
        created_at: row.created_at,
    }))
}

#[server(prefix = "/leptos")]
pub async fn start_timer(item_id: String) -> Result<TimeEntry, ServerFnError> {
    let (pool, user) = require_user().await?;
    let entry = domain::time_entries::start(&pool, &user.id, Some(&item_id))
        .await
        .sfn()?;
    Ok(domain_entry_to_shared(entry))
}

#[server(prefix = "/leptos")]
pub async fn stop_timer() -> Result<Option<TimeEntry>, ServerFnError> {
    let (pool, user) = require_user().await?;
    let entry = domain::time_entries::stop(&pool, &user.id).await.sfn()?;
    Ok(entry.map(domain_entry_to_shared))
}

#[server(prefix = "/leptos")]
pub async fn get_inbox() -> Result<Vec<TimeEntry>, ServerFnError> {
    let (pool, user) = require_user().await?;
    let entries = domain::time_entries::list_inbox(&pool, &user.id)
        .await
        .sfn()?;
    let tz = user_timezone(&pool, &user.id).await?;
    Ok(entries
        .into_iter()
        .map(domain_entry_to_shared)
        .map(|mut entry| {
            // Display-only listing: started_at is shown in the user's timezone.
            entry.started_at = format_datetime_in_tz(&entry.started_at, &tz);
            entry
        })
        .collect())
}

#[server(prefix = "/leptos")]
pub async fn list_all_entries() -> Result<Vec<TimeEntry>, ServerFnError> {
    let (pool, user) = require_user().await?;
    let entries = domain::time_entries::list_all_for_user(&pool, &user.id)
        .await
        .sfn()?;
    let tz = user_timezone(&pool, &user.id).await?;
    Ok(entries
        .into_iter()
        .map(domain_entry_to_shared)
        .map(|mut entry| {
            // Display-only listing: started_at is shown in the user's timezone.
            entry.started_at = format_datetime_in_tz(&entry.started_at, &tz);
            entry
        })
        .collect())
}

#[server(prefix = "/leptos")]
pub async fn log_time(
    item_id: Option<String>,
    started_at: String,
    ended_at: String,
    description: Option<String>,
) -> Result<TimeEntry, ServerFnError> {
    let (pool, user) = require_user().await?;
    let entry = domain::time_entries::log_manual(
        &pool,
        &user.id,
        item_id.as_deref(),
        &started_at,
        &ended_at,
        description.as_deref(),
    )
    .await
    .sfn()?;
    Ok(domain_entry_to_shared(entry))
}

#[server(prefix = "/leptos")]
pub async fn assign_time_entry(
    entry_id: String,
    item_id: String,
) -> Result<TimeEntry, ServerFnError> {
    let (pool, user) = require_user().await?;
    let entry = domain::time_entries::assign(&pool, &user.id, &entry_id, &item_id)
        .await
        .sfn()?;
    Ok(domain_entry_to_shared(entry))
}

#[server(prefix = "/leptos")]
pub async fn delete_time_entry(entry_id: String) -> Result<(), ServerFnError> {
    let (pool, user) = require_user().await?;
    domain::time_entries::delete(&pool, &user.id, &entry_id)
        .await
        .sfn()?;
    Ok(())
}
