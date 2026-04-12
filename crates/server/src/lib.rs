pub mod lists;

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use sqlx::SqlitePool;

// ── AppState ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

// ── UserId extractor (dev: reads X-User-Id header; replaced by auth in C1) ───

#[derive(Clone, Debug)]
pub struct UserId(pub String);

impl<S> FromRequestParts<S> for UserId
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .headers
            .get("x-user-id")
            .and_then(|v| v.to_str().ok())
            .filter(|s| !s.is_empty())
            .map(|s| UserId(s.to_string()))
            .ok_or(AppError::Unauthorized)
    }
}

// ── AppError ──────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("internal: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "missing X-User-Id header".to_string(),
            ),
            AppError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
        };
        (status, msg).into_response()
    }
}

impl From<kartoteka_domain::DomainError> for AppError {
    fn from(e: kartoteka_domain::DomainError) -> Self {
        use kartoteka_domain::DomainError::*;
        match e {
            NotFound(_) => AppError::NotFound,
            Validation(msg) => AppError::BadRequest(msg.to_string()),
            FeatureRequired(f) => AppError::BadRequest(format!("feature required: {f}")),
            Forbidden => AppError::BadRequest("forbidden".into()),
            Internal(msg) => AppError::Internal(msg),
            Db(e) => AppError::Internal(e.to_string()),
        }
    }
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn api_router() -> axum::Router<AppState> {
    // B1 will add containers_router() here when merged with B2
    lists::lists_router()
}
