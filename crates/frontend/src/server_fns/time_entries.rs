use kartoteka_shared::types::{ItemTimeSummary, TimeEntry};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    axum_login::AuthSession, kartoteka_auth::KartotekaBackend, kartoteka_domain as domain,
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
fn domain_summary_to_shared(s: domain::time_entries::ItemTimeSummary) -> ItemTimeSummary {
    ItemTimeSummary {
        total_seconds: s.total_seconds,
        entry_count: s.entry_count,
    }
}

#[cfg(feature = "ssr")]
async fn current_user_id() -> Result<String, ServerFnError> {
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    auth.user
        .map(|u| u.id)
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))
}

#[server(prefix = "/leptos")]
pub async fn get_time_summary(item_id: String) -> Result<ItemTimeSummary, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let uid = current_user_id().await?;
    let summary = domain::time_entries::summary_for_item(&pool, &uid, &item_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(domain_summary_to_shared(summary))
}

#[server(prefix = "/leptos")]
pub async fn get_running_timer() -> Result<Option<TimeEntry>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let uid = current_user_id().await?;
    let entry = kartoteka_db::time_entries::get_running(&pool, &uid)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
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
    let pool = expect_context::<SqlitePool>();
    let uid = current_user_id().await?;
    let entry = domain::time_entries::start(&pool, &uid, Some(&item_id))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(domain_entry_to_shared(entry))
}

#[server(prefix = "/leptos")]
pub async fn stop_timer() -> Result<Option<TimeEntry>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let uid = current_user_id().await?;
    let entry = domain::time_entries::stop(&pool, &uid)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(entry.map(domain_entry_to_shared))
}

#[server(prefix = "/leptos")]
pub async fn get_inbox() -> Result<Vec<TimeEntry>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let uid = current_user_id().await?;
    let entries = domain::time_entries::list_inbox(&pool, &uid)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(entries.into_iter().map(domain_entry_to_shared).collect())
}

#[server(prefix = "/leptos")]
pub async fn list_all_entries() -> Result<Vec<TimeEntry>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let uid = current_user_id().await?;
    let entries = domain::time_entries::list_all_for_user(&pool, &uid)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(entries.into_iter().map(domain_entry_to_shared).collect())
}

#[server(prefix = "/leptos")]
pub async fn log_time(
    item_id: Option<String>,
    started_at: String,
    ended_at: String,
    description: Option<String>,
) -> Result<TimeEntry, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let uid = current_user_id().await?;
    let entry = domain::time_entries::log_manual(
        &pool,
        &uid,
        item_id.as_deref(),
        &started_at,
        &ended_at,
        description.as_deref(),
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(domain_entry_to_shared(entry))
}

#[server(prefix = "/leptos")]
pub async fn assign_time_entry(
    entry_id: String,
    item_id: String,
) -> Result<TimeEntry, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let uid = current_user_id().await?;
    let entry = domain::time_entries::assign(&pool, &uid, &entry_id, &item_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(domain_entry_to_shared(entry))
}

#[server(prefix = "/leptos")]
pub async fn delete_time_entry(entry_id: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let uid = current_user_id().await?;
    domain::time_entries::delete(&pool, &uid, &entry_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}
