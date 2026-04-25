use crate::{AppError, AppState, extractors::UserId};
use axum::extract::Request;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use kartoteka_auth::{AuthSession, LoginCredentials};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub role: String,
}

#[derive(Deserialize)]
pub struct SetConfigValueRequest {
    pub value: String,
}

#[derive(Serialize)]
pub struct ConfigEntry {
    pub key: String,
    pub value: String,
}

#[derive(Deserialize)]
pub struct TwoFaRequest {
    pub code: String,
}

#[derive(Deserialize)]
pub struct TotpCodeRequest {
    pub code: String,
}

#[derive(Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
    pub scope: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Serialize)]
pub struct TokenCreatedResponse {
    pub id: String,
    pub token: String,
    pub name: String,
    pub scope: String,
}

#[derive(Serialize)]
pub struct TokenListItem {
    pub id: String,
    pub name: String,
    pub scope: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

use kartoteka_shared::constants::{
    SESSION_MAX_2FA_ATTEMPTS, SESSION_PENDING_2FA_ATTEMPTS_KEY, SESSION_PENDING_USER_KEY,
    SESSION_RETURN_TO_KEY,
};
const PENDING_USER_KEY: &str = SESSION_PENDING_USER_KEY;
const PENDING_2FA_ATTEMPTS_KEY: &str = SESSION_PENDING_2FA_ATTEMPTS_KEY;
const MAX_2FA_ATTEMPTS: u32 = SESSION_MAX_2FA_ATTEMPTS;
const RETURN_TO_KEY: &str = SESSION_RETURN_TO_KEY;

/// Extract a bearer token from the Authorization header.
fn extract_bearer_token(headers: &axum::http::HeaderMap) -> Option<&str> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

/// Unified auth middleware — Bearer JWT first, session fallback.
///
/// Bearer behaviour:
///   Valid JWT + scope "full" → inject UserId, proceed
///   Valid JWT + other scope → 403 (wrong scope for /api/*)
///   Invalid/expired/revoked JWT → 401 (does NOT fall through to session)
///
/// Session behaviour (no Bearer header):
///   Active session → inject UserId, proceed
///   No session → 401
pub async fn require_auth(
    State(state): State<AppState>,
    auth_session: AuthSession,
    mut req: Request,
    next: Next,
) -> Response {
    if let Some(token) = extract_bearer_token(req.headers()) {
        return match kartoteka_domain::auth::validate_jwt(&state.pool, token, &state.signing_secret)
            .await
        {
            Ok(ctx) if ctx.scope == "full" => {
                req.extensions_mut().insert(UserId(ctx.user_id));
                next.run(req).await
            }
            Ok(_) => StatusCode::FORBIDDEN.into_response(),
            Err(_) => StatusCode::UNAUTHORIZED.into_response(),
        };
    }

    match auth_session.user {
        Some(ref user) => {
            req.extensions_mut().insert(UserId(user.id.clone()));
            next.run(req).await
        }
        None => StatusCode::UNAUTHORIZED.into_response(),
    }
}

/// Require admin role. Returns 403 if authenticated but not admin, 401 if not auth'd.
pub async fn require_admin(auth_session: AuthSession, req: Request, next: Next) -> Response {
    match &auth_session.user {
        Some(user) if user.role == "admin" => next.run(req).await,
        Some(_) => StatusCode::FORBIDDEN.into_response(),
        None => StatusCode::UNAUTHORIZED.into_response(),
    }
}

/// POST /auth/register
#[tracing::instrument(skip_all, fields(action = "register"))]
pub async fn register(
    State(state): State<AppState>,
    mut auth_session: AuthSession,
    Json(req): Json<RegisterRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user_info = kartoteka_domain::auth::register(
        &state.pool,
        &req.email,
        &req.password,
        req.name.as_deref(),
    )
    .await?;

    let creds = LoginCredentials {
        email: req.email,
        password: req.password,
    };
    match auth_session.authenticate(creds).await {
        Ok(Some(user)) => {
            if let Err(e) = auth_session.login(&user).await {
                tracing::warn!("auto-login after registration failed: {e}");
            }
        }
        Ok(None) => tracing::warn!(
            "auto-login after registration: user not found immediately after creation"
        ),
        Err(e) => tracing::warn!("auto-login after registration: auth error: {e}"),
    }

    Ok((
        StatusCode::CREATED,
        Json(UserResponse {
            id: user_info.id,
            email: user_info.email,
            name: user_info.name,
            role: user_info.role,
        }),
    ))
}

/// POST /auth/login
///
/// If the user has TOTP enabled, stores `pending_user_id` in the raw session and
/// returns `{"status": "2fa_required"}`. Client must then POST /auth/2fa to complete.
#[tracing::instrument(skip_all, fields(action = "login"))]
pub async fn login(
    mut auth_session: AuthSession,
    session: Session,
    State(state): State<AppState>,
    Json(creds): Json<LoginCredentials>,
) -> impl IntoResponse {
    let user = match auth_session.authenticate(creds).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "invalid_credentials"})),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("auth backend error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match kartoteka_domain::auth::is_totp_enabled(&state.pool, &user.id).await {
        Ok(true) => {
            if let Err(e) = session.insert(PENDING_USER_KEY, &user.id).await {
                tracing::error!("session insert failed: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
            return Json(serde_json::json!({"status": "2fa_required"})).into_response();
        }
        Ok(false) => {}
        Err(e) => {
            tracing::error!("totp check failed: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    if let Err(e) = auth_session.login(&user).await {
        tracing::error!("session write failed during login: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    Json(serde_json::json!({
        "status": "ok",
        "user": {
            "id": user.id,
            "email": user.email,
            "name": user.name,
            "role": user.role,
        }
    }))
    .into_response()
}

/// POST /auth/logout
#[tracing::instrument(skip_all, fields(action = "logout"))]
pub async fn logout(mut auth_session: AuthSession) -> impl IntoResponse {
    if let Err(e) = auth_session.logout().await {
        tracing::error!("session invalidation failed during logout: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    Json(serde_json::json!({"status": "ok"})).into_response()
}

/// POST /auth/2fa
///
/// Reads `pending_user_id` from session, verifies TOTP code, completes login.
#[tracing::instrument(skip_all, fields(action = "verify_2fa"))]
pub async fn verify_2fa(
    mut auth_session: AuthSession,
    session: Session,
    State(state): State<AppState>,
    Json(req): Json<TwoFaRequest>,
) -> impl IntoResponse {
    let user_id: String = match session.get(PENDING_USER_KEY).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "no_pending_2fa"})),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("session read failed: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match kartoteka_domain::auth::check_totp_code(&state.pool, &user_id, &req.code).await {
        Ok(true) => {}
        Ok(false) => {
            // Track attempts in the session; after MAX_2FA_ATTEMPTS clear the
            // pending login so the attacker must re-authenticate with password.
            let attempts: u32 = session
                .get(PENDING_2FA_ATTEMPTS_KEY)
                .await
                .ok()
                .flatten()
                .unwrap_or(0)
                + 1;
            if attempts >= MAX_2FA_ATTEMPTS {
                if let Err(e) = session.remove::<String>(PENDING_USER_KEY).await {
                    tracing::warn!("failed to clear {PENDING_USER_KEY} after lockout: {e}");
                }
                if let Err(e) = session.remove::<u32>(PENDING_2FA_ATTEMPTS_KEY).await {
                    tracing::warn!("failed to clear {PENDING_2FA_ATTEMPTS_KEY}: {e}");
                }
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({"error": "too_many_attempts"})),
                )
                    .into_response();
            }
            if let Err(e) = session.insert(PENDING_2FA_ATTEMPTS_KEY, attempts).await {
                tracing::warn!("failed to persist {PENDING_2FA_ATTEMPTS_KEY}: {e}");
            }
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "invalid_code"})),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("totp check error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    let user = match kartoteka_auth::get_user_by_id(&state.pool, &user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "user_not_found"})),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("user lookup failed: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if let Err(e) = session.remove::<String>(PENDING_USER_KEY).await {
        tracing::warn!("failed to remove {PENDING_USER_KEY} from session: {e}");
    }
    if let Err(e) = session.remove::<u32>(PENDING_2FA_ATTEMPTS_KEY).await {
        tracing::warn!("failed to remove {PENDING_2FA_ATTEMPTS_KEY} from session: {e}");
    }
    let return_to: Option<String> = session.remove(RETURN_TO_KEY).await.unwrap_or(None);

    if let Err(e) = auth_session.login(&user).await {
        tracing::error!("session write failed during 2fa login: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    Json(serde_json::json!({
        "status": "ok",
        "user": {
            "id": user.id,
            "email": user.email,
            "name": user.name,
            "role": user.role,
        },
        "return_to": return_to,
    }))
    .into_response()
}

/// POST /auth/totp/setup (authenticated)
#[tracing::instrument(skip_all, fields(action = "totp_setup"))]
pub async fn totp_setup(
    auth_session: AuthSession,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = auth_session.user.ok_or(AppError::Unauthorized)?;
    let setup = kartoteka_domain::auth::setup_totp(&state.pool, &user.id, &user.email).await?;
    Ok(Json(serde_json::json!({
        "secret": setup.secret,
        "otpauth_url": setup.otpauth_url,
    })))
}

/// POST /auth/totp/verify (authenticated)
#[tracing::instrument(skip_all, fields(action = "totp_verify_setup"))]
pub async fn totp_verify_setup(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Json(req): Json<TotpCodeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = auth_session.user.ok_or(AppError::Unauthorized)?;
    kartoteka_domain::auth::verify_totp_setup(&state.pool, &user.id, &req.code).await?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

/// DELETE /auth/totp (authenticated)
#[tracing::instrument(skip_all, fields(action = "totp_delete"))]
pub async fn totp_delete(
    auth_session: AuthSession,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = auth_session.user.ok_or(AppError::Unauthorized)?;
    kartoteka_domain::auth::disable_totp(&state.pool, &user.id).await?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

/// GET /api/server-config (admin only)
#[tracing::instrument(skip_all, fields(action = "get_server_config"))]
pub async fn get_server_config(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let enabled = kartoteka_domain::auth::is_registration_enabled(&state.pool).await?;
    Ok(Json(serde_json::json!({"registration_enabled": enabled})))
}

/// PUT /api/server-config/{key} (admin only)
#[tracing::instrument(skip_all, fields(action = "set_server_config"))]
pub async fn set_server_config(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<SetConfigValueRequest>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::auth::set_server_config(&state.pool, &key, &req.value).await?;
    Ok(Json(ConfigEntry {
        key,
        value: req.value,
    }))
}

const DEFAULT_TOKEN_TTL_DAYS: i64 = 90;
const MAX_TOKEN_TTL_DAYS: i64 = 365;

/// POST /auth/tokens — create a personal access token (session auth required)
#[tracing::instrument(skip_all, fields(action = "create_token"))]
pub async fn create_token_handler(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Json(req): Json<CreateTokenRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = auth_session.user.ok_or(AppError::Unauthorized)?;
    let scope = req.scope.as_deref().unwrap_or("full");

    const ALLOWED_SCOPES: &[&str] = &["full", "readonly"];
    if !ALLOWED_SCOPES.contains(&scope) {
        return Err(AppError::Validation(format!(
            "invalid scope '{scope}': must be one of: full, readonly"
        )));
    }

    let now = chrono::Utc::now();
    let expires_at = match req.expires_at.as_deref() {
        Some(s) => {
            let dt = chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map_err(|_| {
                    AppError::Validation("invalid expires_at: expected RFC3339".to_string())
                })?;
            if dt <= now {
                return Err(AppError::Validation(
                    "expires_at must be in the future".to_string(),
                ));
            }
            let max = now + chrono::Duration::days(MAX_TOKEN_TTL_DAYS);
            if dt > max {
                return Err(AppError::Validation(format!(
                    "expires_at exceeds maximum of {MAX_TOKEN_TTL_DAYS} days"
                )));
            }
            dt
        }
        None => now + chrono::Duration::days(DEFAULT_TOKEN_TTL_DAYS),
    };

    let created = kartoteka_domain::auth::create_token(
        &state.pool,
        &state.signing_secret,
        &user.id,
        &req.name,
        scope,
        Some(expires_at),
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(TokenCreatedResponse {
            id: created.id,
            token: created.token,
            name: created.name,
            scope: created.scope,
        }),
    ))
}

/// GET /auth/tokens — list tokens for authenticated user (session auth required)
#[tracing::instrument(skip_all, fields(action = "list_tokens"))]
pub async fn list_tokens_handler(
    auth_session: AuthSession,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = auth_session.user.ok_or(AppError::Unauthorized)?;
    let tokens = kartoteka_domain::auth::list_tokens(&state.pool, &user.id).await?;
    let items: Vec<TokenListItem> = tokens
        .into_iter()
        .map(|row| TokenListItem {
            id: row.id,
            name: row.name,
            scope: row.scope,
            last_used_at: row.last_used_at,
            expires_at: row.expires_at,
            created_at: row.created_at,
        })
        .collect();
    Ok(Json(items))
}

/// DELETE /auth/tokens/{id} — revoke a token (session auth required)
#[tracing::instrument(skip_all, fields(action = "delete_token", token_id = %id))]
pub async fn delete_token_handler(
    auth_session: AuthSession,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let user = auth_session.user.ok_or(AppError::Unauthorized)?;
    kartoteka_domain::auth::revoke_token(&state.pool, &id, &user.id).await?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

/// Router for /auth/* (public + self-guarded authenticated routes)
pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/2fa", post(verify_2fa))
        .route("/totp/setup", post(totp_setup))
        .route("/totp/verify", post(totp_verify_setup))
        .route("/totp", delete(totp_delete))
        .route(
            "/tokens",
            post(create_token_handler).get(list_tokens_handler),
        )
        .route("/tokens/{id}", delete(delete_token_handler))
}

/// Router for /api/server-config (admin required — caller wraps with require_admin)
pub fn server_config_router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_server_config))
        .route("/{key}", put(set_server_config))
}
