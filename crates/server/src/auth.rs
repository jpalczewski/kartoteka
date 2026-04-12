use crate::{AppError, AppState, extractors::UserId};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use axum::extract::Request;
use kartoteka_auth::{AuthSession, LoginCredentials};
use serde::{Deserialize, Serialize};

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

/// Require authenticated session. Inserts UserId into extensions. Returns 401 if not auth'd.
pub async fn require_auth(auth_session: AuthSession, mut req: Request, next: Next) -> Response {
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
#[tracing::instrument(skip(state, auth_session, req))]
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

    // Auto-login after registration
    let creds = LoginCredentials { email: req.email, password: req.password };
    match auth_session.authenticate(creds).await {
        Ok(Some(user)) => {
            if let Err(e) = auth_session.login(&user).await {
                tracing::warn!("auto-login after registration failed: {e}");
            }
        }
        Ok(None) => tracing::warn!("auto-login after registration: user not found immediately after creation"),
        Err(e) => tracing::warn!("auto-login after registration: auth error: {e}"),
    }

    Ok((StatusCode::CREATED, Json(UserResponse {
        id: user_info.id,
        email: user_info.email,
        name: user_info.name,
        role: user_info.role,
    })))
}

/// POST /auth/login
#[tracing::instrument(skip(auth_session, creds))]
pub async fn login(
    mut auth_session: AuthSession,
    Json(creds): Json<LoginCredentials>,
) -> impl IntoResponse {
    match auth_session.authenticate(creds).await {
        Ok(Some(user)) => {
            let _ = auth_session.login(&user).await;
            Json(serde_json::json!({
                "status": "ok",
                "user": {
                    "id": user.id,
                    "email": user.email,
                    "name": user.name,
                    "role": user.role,
                }
            })).into_response()
        }
        Ok(None) => (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "invalid credentials"}))).into_response(),
        Err(e) => {
            tracing::error!("auth backend error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// POST /auth/logout
#[tracing::instrument(skip(auth_session))]
pub async fn logout(mut auth_session: AuthSession) -> impl IntoResponse {
    let _ = auth_session.logout().await;
    Json(serde_json::json!({"status": "ok"}))
}

/// GET /api/server-config (admin only)
#[tracing::instrument(skip(state))]
pub async fn get_server_config(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let enabled = kartoteka_domain::auth::is_registration_enabled(&state.pool).await?;
    Ok(Json(serde_json::json!({"registration_enabled": enabled})))
}

/// PUT /api/server-config/{key} (admin only)
#[tracing::instrument(skip(state, req))]
pub async fn set_server_config(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(req): Json<SetConfigValueRequest>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::auth::set_server_config(&state.pool, &key, &req.value).await?;
    Ok(Json(ConfigEntry { key, value: req.value }))
}

/// Router for /auth/* (no auth required)
pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
}

/// Router for /api/server-config (admin required — caller wraps with require_admin)
pub fn server_config_router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_server_config))
        .route("/{key}", put(set_server_config))
}
