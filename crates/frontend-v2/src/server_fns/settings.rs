use kartoteka_shared::types::{TokenCreated, TokenInfo, UserSetting};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    axum_login::AuthSession,
    kartoteka_auth::KartotekaBackend,
    kartoteka_domain as domain,
    sqlx::SqlitePool,
};

/// Newtype wrapper for the HMAC signing secret, provided as context by the server.
#[cfg(feature = "ssr")]
#[derive(Clone)]
pub struct SigningSecret(pub String);

/// All settings for the current user as key-value pairs.
#[server(prefix = "/leptos")]
pub async fn get_all_settings() -> Result<Vec<UserSetting>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let settings = domain::settings::list_all(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(settings
        .into_iter()
        .map(|s| UserSetting {
            key: s.key,
            value: s.value,
            updated_at: s.updated_at,
        })
        .collect())
}

/// Set (upsert) a setting value.
#[server(prefix = "/leptos")]
pub async fn set_setting(key: String, value: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::settings::set(&pool, &user.id, &key, &value)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

/// List personal API tokens for the current user (JWT value not included).
#[server(prefix = "/leptos")]
pub async fn list_tokens() -> Result<Vec<TokenInfo>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let rows = domain::auth::list_tokens(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|r| TokenInfo {
            id: r.id,
            name: r.name,
            scope: r.scope,
            last_used_at: r.last_used_at,
            expires_at: r.expires_at,
            created_at: r.created_at,
        })
        .collect())
}

/// Create a new personal API token. Returns the JWT string ONCE — store it now or lose it.
#[server(prefix = "/leptos")]
pub async fn create_token_sf(name: String, scope: String) -> Result<TokenCreated, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let signing_secret = expect_context::<SigningSecret>().0;
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let created = domain::auth::create_token(
        &pool,
        &signing_secret,
        &user.id,
        &name,
        &scope,
        None,
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(TokenCreated {
        id: created.id,
        token: created.token,
        name: created.name,
        scope: created.scope,
    })
}

/// Revoke (delete) a personal API token by id.
#[server(prefix = "/leptos")]
pub async fn revoke_token_sf(token_id: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::auth::revoke_token(&pool, &token_id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

/// Is registration open? (public — no auth required, used on signup page).
#[server(prefix = "/leptos")]
pub async fn is_reg_enabled() -> Result<bool, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    domain::auth::is_registration_enabled(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Toggle registration on/off. Admin only.
#[server(prefix = "/leptos")]
pub async fn set_reg_enabled(enabled: bool) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    if user.role != "admin" {
        return Err(ServerFnError::new("forbidden".to_string()));
    }
    let value = if enabled { "true" } else { "false" };
    domain::auth::set_server_config(&pool, "registration_enabled", value)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
