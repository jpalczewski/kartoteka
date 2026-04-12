use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use kartoteka_domain::DomainError;

#[derive(Debug)]
pub enum AppError {
    NotFound(&'static str),
    Validation(String),
    Forbidden,
    Unauthorized,
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg).into_response(),
            AppError::Validation(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg).into_response(),
            AppError::Forbidden => StatusCode::FORBIDDEN.into_response(),
            AppError::Unauthorized => StatusCode::UNAUTHORIZED.into_response(),
            AppError::Internal(msg) => {
                tracing::error!("internal server error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
            }
        }
    }
}

impl From<DomainError> for AppError {
    fn from(e: DomainError) -> Self {
        match e {
            DomainError::NotFound(msg) => AppError::NotFound(msg),
            DomainError::Validation(msg) => AppError::Validation(msg.to_string()),
            DomainError::FeatureRequired(f) => AppError::Validation(format!("feature required: {f}")),
            DomainError::Forbidden => AppError::Forbidden,
            DomainError::Internal(msg) => AppError::Internal(msg),
            DomainError::Db(e) => AppError::Internal(e.to_string()),
        }
    }
}
