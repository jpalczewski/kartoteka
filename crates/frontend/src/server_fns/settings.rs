use kartoteka_shared::Locale;
use kartoteka_shared::types::{TokenCreated, TokenInfo, UserSetting};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    crate::server_fns::utils::{SfnResultExt, require_admin, require_user},
    axum_login::AuthSession,
    kartoteka_auth::KartotekaBackend,
    kartoteka_domain as domain,
    sqlx::SqlitePool,
};

/// Newtype wrapper for the HMAC signing secret, provided as context by the server.
#[cfg(feature = "ssr")]
#[derive(Clone)]
pub struct SigningSecret(pub String);

/// Returns the authenticated user's stored locale for leptos-fluent initialization.
/// Returns None for unauthenticated requests (landing page, login page).
#[server(prefix = "/leptos")]
pub async fn get_user_locale_sf() -> Result<Option<String>, ServerFnError> {
    let Some(pool) = use_context::<SqlitePool>() else {
        return Ok(None);
    };
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let Some(user) = auth.user else {
        return Ok(None);
    };
    let locale = domain::preferences::get_locale(&pool, &user.id)
        .await
        .sfn()?;
    Ok(Some(locale.to_string()))
}

/// All settings for the current user as key-value pairs.
#[server(prefix = "/leptos")]
pub async fn get_all_settings() -> Result<Vec<UserSetting>, ServerFnError> {
    let (pool, user) = require_user().await?;
    let settings = domain::settings::list_all(&pool, &user.id).await.sfn()?;
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
    let (pool, user) = require_user().await?;
    domain::settings::set(&pool, &user.id, &key, &value)
        .await
        .sfn()?;
    Ok(())
}

/// List personal API tokens for the current user (JWT value not included).
#[server(prefix = "/leptos")]
pub async fn list_tokens() -> Result<Vec<TokenInfo>, ServerFnError> {
    let (pool, user) = require_user().await?;
    let rows = domain::auth::list_tokens(&pool, &user.id).await.sfn()?;
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
/// Scope and expiry are validated in `domain::auth` (default: full scope, 90-day expiry).
#[server(prefix = "/leptos")]
pub async fn create_token_sf(name: String, scope: String) -> Result<TokenCreated, ServerFnError> {
    let signing_secret = expect_context::<SigningSecret>().0;
    let (pool, user) = require_user().await?;
    let params = domain::auth::CreateTokenParams {
        name,
        scope: Some(scope),
        expires_at: None,
    };
    let created = domain::auth::create_personal_token(&pool, &signing_secret, &user.id, params)
        .await
        .sfn()?;
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
    let (pool, user) = require_user().await?;
    domain::auth::revoke_token(&pool, &token_id, &user.id)
        .await
        .sfn()?;
    Ok(())
}

/// Is registration open? (public — no auth required, used on signup page).
#[server(prefix = "/leptos")]
pub async fn is_reg_enabled() -> Result<bool, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    domain::auth::is_registration_enabled(&pool).await.sfn()
}

/// Toggle registration on/off. Admin only.
#[server(prefix = "/leptos")]
pub async fn set_reg_enabled(enabled: bool) -> Result<(), ServerFnError> {
    let (pool, _) = require_admin().await?;
    let value = if enabled { "true" } else { "false" };
    domain::auth::set_server_config(&pool, "registration_enabled", value)
        .await
        .sfn()
}

#[server(prefix = "/leptos")]
pub async fn set_locale_sf(locale: Locale) -> Result<(), ServerFnError> {
    let (pool, user) = require_user().await?;
    domain::preferences::set_locale(&pool, &user.id, locale)
        .await
        .sfn()
}

/// Combined settings page data: settings list + is_admin flag + token list + reg_enabled.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SettingsPageData {
    pub settings: Vec<UserSetting>,
    pub locale: Locale,
    pub is_admin: bool,
    pub tokens: Vec<TokenInfo>,
    pub reg_enabled: bool,
}

#[server(prefix = "/leptos")]
pub async fn get_settings_page_data() -> Result<SettingsPageData, ServerFnError> {
    let (pool, user) = require_user().await?;

    let (settings, locale, token_rows, reg_enabled) = tokio::try_join!(
        domain::settings::list_all(&pool, &user.id),
        domain::preferences::get_locale(&pool, &user.id),
        domain::auth::list_tokens(&pool, &user.id),
        domain::auth::is_registration_enabled(&pool),
    )
    .sfn()?;

    Ok(SettingsPageData {
        settings: settings
            .into_iter()
            .map(|s| UserSetting {
                key: s.key,
                value: s.value,
                updated_at: s.updated_at,
            })
            .collect(),
        locale,
        is_admin: user.role == "admin",
        tokens: token_rows
            .into_iter()
            .map(|r| TokenInfo {
                id: r.id,
                name: r.name,
                scope: r.scope,
                last_used_at: r.last_used_at,
                expires_at: r.expires_at,
                created_at: r.created_at,
            })
            .collect(),
        reg_enabled,
    })
}

/// Today's date (`YYYY-MM-DD`) in the signed-in user's timezone.
///
/// Item views compare dates against this, not the browser clock, so the server render and
/// the hydrated page label "today"/"overdue" the same way.
#[server(prefix = "/leptos")]
pub async fn get_user_today() -> Result<String, ServerFnError> {
    let (pool, user) = require_user().await?;
    domain::items::user_today(&pool, &user.id).await.sfn()
}
