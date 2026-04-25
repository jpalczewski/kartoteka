use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use kartoteka_db::DbError;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("invalid_request: {0}")]
    InvalidRequest(&'static str),
    #[error("invalid_client")]
    InvalidClient,
    #[error("invalid_grant: {0}")]
    InvalidGrant(&'static str),
    #[error("unsupported_grant_type")]
    UnsupportedGrantType,
    #[error("access_denied")]
    AccessDenied,
    #[error("internal: {0}")]
    Internal(String),
    #[error(transparent)]
    Db(#[from] DbError),
}

impl IntoResponse for OAuthError {
    fn into_response(self) -> Response {
        let (status, code, desc) = match &self {
            OAuthError::InvalidRequest(d) => {
                (StatusCode::BAD_REQUEST, "invalid_request", d.to_string())
            }
            OAuthError::InvalidClient => (
                StatusCode::UNAUTHORIZED,
                "invalid_client",
                "unknown or unauthorized client".into(),
            ),
            OAuthError::InvalidGrant(d) => {
                (StatusCode::BAD_REQUEST, "invalid_grant", d.to_string())
            }
            OAuthError::UnsupportedGrantType => (
                StatusCode::BAD_REQUEST,
                "unsupported_grant_type",
                "grant_type not supported".into(),
            ),
            OAuthError::AccessDenied => (
                StatusCode::FORBIDDEN,
                "access_denied",
                "user denied consent".into(),
            ),
            OAuthError::Internal(m) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "server_error", m.clone())
            }
            OAuthError::Db(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "server_error",
                e.to_string(),
            ),
        };
        (
            status,
            Json(json!({ "error": code, "error_description": desc })),
        )
            .into_response()
    }
}

impl From<jsonwebtoken::errors::Error> for OAuthError {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        OAuthError::Internal(format!("jwt: {e}"))
    }
}
