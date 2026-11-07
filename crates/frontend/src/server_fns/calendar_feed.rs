//! Server functions for the iCal feed settings: subscription tokens and the
//! user's default event reminders.
//!
//! The domain types are not WASM-safe, so everything crossing the wire uses the
//! DTOs in [`kartoteka_shared::types`].

use kartoteka_shared::ReminderDefaults;
use kartoteka_shared::types::{CalendarTokenCreatedDto, CalendarTokenInfo};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    crate::server_fns::utils::format_datetime_in_tz, axum_login::AuthSession,
    kartoteka_auth::KartotekaBackend, kartoteka_domain as domain, sqlx::SqlitePool,
};

/// List the current user's calendar subscription tokens, with the scoped list's
/// name resolved for display.
#[server(prefix = "/leptos")]
pub async fn list_calendar_tokens_sf() -> Result<Vec<CalendarTokenInfo>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let rows = domain::calendar::list_tokens(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let tz = domain::preferences::get(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .timezone;

    let mut tokens = Vec::with_capacity(rows.len());
    for row in rows {
        // A token may point at a sublist, so resolve the name per token instead
        // of matching against the root-list listing.
        let list_name = match row.list_id.as_deref() {
            Some(list_id) => domain::lists::get_one(&pool, list_id, &user.id)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?
                .map(|list| list.name),
            None => None,
        };
        tokens.push(CalendarTokenInfo {
            id: row.id,
            name: row.name,
            list_id: row.list_id,
            list_name,
            last_used_at: row.last_used_at.map(|at| format_datetime_in_tz(&at, &tz)),
            created_at: format_datetime_in_tz(&row.created_at, &tz),
        });
    }
    Ok(tokens)
}

/// Create a calendar subscription token. The feed URL is built from the
/// returned `token`, which is shown once and never stored in plaintext.
#[server(prefix = "/leptos")]
pub async fn create_calendar_token_sf(
    name: String,
    list_id: Option<String>,
) -> Result<CalendarTokenCreatedDto, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let created = domain::calendar::create_token(&pool, &user.id, &name, list_id.as_deref())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(CalendarTokenCreatedDto {
        id: created.id,
        name: created.name,
        token: created.token,
        list_id: created.list_id,
    })
}

/// Revoke (delete) a calendar subscription token by id. Every client using that
/// feed URL stops receiving updates.
#[server(prefix = "/leptos")]
pub async fn revoke_calendar_token_sf(id: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::calendar::revoke_token(&pool, &user.id, &id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// The current user's default reminders (minutes before each date field).
#[server(prefix = "/leptos")]
pub async fn get_reminder_defaults_sf() -> Result<ReminderDefaults, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::calendar::get_reminder_defaults(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Persist the default reminders. The domain validates the shape (at most 5
/// offsets per field, each at most 28 days).
#[server(prefix = "/leptos")]
pub async fn set_reminder_defaults_sf(value: ReminderDefaults) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::calendar::set_reminder_defaults(&pool, &user.id, &value)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
