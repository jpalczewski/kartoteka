use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use {
    axum_login::AuthSession,
    kartoteka_auth::{KartotekaBackend, LoginCredentials},
    kartoteka_domain as domain,
    kartoteka_shared::constants::{
        SESSION_MAX_2FA_ATTEMPTS, SESSION_PENDING_2FA_ATTEMPTS_KEY, SESSION_PENDING_USER_KEY,
        SESSION_RETURN_TO_KEY,
    },
    sqlx::SqlitePool,
    tower_sessions::Session,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LoginOutcome {
    Ok,
    TwoFaRequired,
}

/// Current user's display name — used by the navbar.
#[server(prefix = "/leptos")]
pub async fn get_nav_data() -> Result<String, ServerFnError> {
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    Ok(user.name.unwrap_or(user.email))
}

/// Authenticate with email + password.
/// Returns `TwoFaRequired` if the account has TOTP enabled — the caller must then
/// present a TOTP code via `do_verify_2fa`. On `Ok`, the session is fully established
/// and the server issues a redirect to `return_to` (or `/`).
#[server(prefix = "/leptos")]
pub async fn do_login(
    email: String,
    password: String,
    return_to: Option<String>,
) -> Result<LoginOutcome, ServerFnError> {
    let mut auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;

    let pool = expect_context::<SqlitePool>();
    let session = leptos_axum::extract::<Session>()
        .await
        .map_err(|_| ServerFnError::new("session extraction failed".to_string()))?;

    let credentials = LoginCredentials { email, password };
    let user = auth
        .authenticate(credentials)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("invalid credentials".to_string()))?;

    let totp_enabled = domain::auth::is_totp_enabled(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if totp_enabled {
        session
            .insert(SESSION_PENDING_USER_KEY, &user.id)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        if let Some(ref rt) = return_to {
            if rt.starts_with('/') && !rt.starts_with("//") {
                let _ = session.insert(SESSION_RETURN_TO_KEY, rt).await;
            }
        }
        return Ok(LoginOutcome::TwoFaRequired);
    }

    auth.login(&user)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let redirect_to = return_to
        .filter(|r| r.starts_with('/') && !r.starts_with("//"))
        .unwrap_or_else(|| "/".to_string());
    leptos_axum::redirect(&redirect_to);
    Ok(LoginOutcome::Ok)
}

/// Verify a TOTP code after `do_login` returned `TwoFaRequired`.
/// On success the session is fully established and the server issues a redirect.
#[server(prefix = "/leptos")]
pub async fn do_verify_2fa(code: String) -> Result<(), ServerFnError> {
    let mut auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;

    let pool = expect_context::<SqlitePool>();
    let session = leptos_axum::extract::<Session>()
        .await
        .map_err(|_| ServerFnError::new("session extraction failed".to_string()))?;

    let user_id: String = session
        .get(SESSION_PENDING_USER_KEY)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("no_pending_2fa".to_string()))?;

    let valid = domain::auth::check_totp_code(&pool, &user_id, &code)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if !valid {
        let attempts: u32 = session
            .get(SESSION_PENDING_2FA_ATTEMPTS_KEY)
            .await
            .ok()
            .flatten()
            .unwrap_or(0)
            + 1;
        if attempts >= SESSION_MAX_2FA_ATTEMPTS {
            let _ = session.remove::<String>(SESSION_PENDING_USER_KEY).await;
            let _ = session
                .remove::<u32>(SESSION_PENDING_2FA_ATTEMPTS_KEY)
                .await;
            return Err(ServerFnError::new("too_many_attempts".to_string()));
        }
        let _ = session
            .insert(SESSION_PENDING_2FA_ATTEMPTS_KEY, attempts)
            .await;
        return Err(ServerFnError::new("invalid_code".to_string()));
    }

    let user = kartoteka_auth::get_user_by_id(&pool, &user_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("user_not_found".to_string()))?;

    let return_to: Option<String> = session.remove(SESSION_RETURN_TO_KEY).await.unwrap_or(None);
    let _ = session.remove::<String>(SESSION_PENDING_USER_KEY).await;
    let _ = session
        .remove::<u32>(SESSION_PENDING_2FA_ATTEMPTS_KEY)
        .await;

    auth.login(&user)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let redirect_to = return_to
        .filter(|r| r.starts_with('/') && !r.starts_with("//"))
        .unwrap_or_else(|| "/".to_string());
    leptos_axum::redirect(&redirect_to);
    Ok(())
}

/// Clear the session and redirect to /login.
#[server(prefix = "/leptos")]
pub async fn do_logout() -> Result<(), ServerFnError> {
    let mut auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;

    auth.logout()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    leptos_axum::redirect("/login");
    Ok(())
}

/// Register a new account. Redirects to /login on success.
#[server(prefix = "/leptos")]
pub async fn do_register(
    email: String,
    password: String,
    name: Option<String>,
) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    domain::auth::register(&pool, &email, &password, name.as_deref())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    leptos_axum::redirect("/login");
    Ok(())
}
