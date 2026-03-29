use serde::Serialize;
use std::fmt;
use worker::Response;

/// Typed API error with automatic log-level mapping.
pub enum ApiError {
    /// 404 — resource not found. Log at DEBUG (client mistake).
    NotFound(String),
    /// 403 — forbidden. Log at WARN (suspicious).
    Forbidden,
    /// 400 — bad request. Log at DEBUG (client mistake).
    BadRequest(String),
    /// 500 — internal error. Log at ERROR (our problem).
    Internal(String),
}

/// Convenience alias for handler return types.
pub type ApiResult<T> = Result<T, ApiError>;

impl ApiError {
    pub fn status_code(&self) -> u16 {
        match self {
            Self::NotFound(_) => 404,
            Self::Forbidden => 403,
            Self::BadRequest(_) => 400,
            Self::Internal(_) => 500,
        }
    }

    pub fn log_level(&self) -> tracing::Level {
        match self {
            Self::NotFound(_) | Self::BadRequest(_) => tracing::Level::DEBUG,
            Self::Forbidden => tracing::Level::WARN,
            Self::Internal(_) => tracing::Level::ERROR,
        }
    }

    pub fn code(&self) -> &str {
        match self {
            Self::NotFound(c) | Self::BadRequest(c) | Self::Internal(c) => c.as_str(),
            Self::Forbidden => "forbidden",
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(c) => write!(f, "not found: {c}"),
            Self::Forbidden => write!(f, "forbidden"),
            Self::BadRequest(c) => write!(f, "bad request: {c}"),
            Self::Internal(c) => write!(f, "internal error: {c}"),
        }
    }
}

impl From<worker::Error> for ApiError {
    fn from(e: worker::Error) -> Self {
        Self::Internal(e.to_string())
    }
}

#[derive(Serialize)]
struct ErrorBody {
    code: String,
    status: u16,
}

/// Convert `ApiResult<T>` to `worker::Result<Response>`, logging at the appropriate level.
pub fn into_response<T: Serialize>(result: ApiResult<T>, status: u16) -> worker::Result<Response> {
    match result {
        Ok(data) => {
            tracing::info!("success");
            Response::from_json(&data).map(|r| r.with_status(status))
        }
        Err(ref e) => {
            match e.log_level() {
                tracing::Level::ERROR => tracing::error!(error = %e, "failed"),
                tracing::Level::WARN => tracing::warn!(error = %e, "failed"),
                _ => tracing::debug!(error = %e, "failed"),
            }
            let body = ErrorBody {
                code: e.code().to_string(),
                status: e.status_code(),
            };
            Response::from_json(&body).map(|r| r.with_status(e.status_code()))
        }
    }
}

/// Shortcut for 200 OK response with logging.
pub fn ok_response<T: Serialize>(result: ApiResult<T>) -> worker::Result<Response> {
    into_response(result, 200)
}

/// Shortcut for 201 Created response with logging.
pub fn created_response<T: Serialize>(result: ApiResult<T>) -> worker::Result<Response> {
    into_response(result, 201)
}

/// Shortcut for 204 No Content response with logging.
pub fn no_content_response(result: ApiResult<()>) -> worker::Result<Response> {
    match result {
        Ok(()) => {
            tracing::info!("success");
            Ok(Response::empty()?.with_status(204))
        }
        Err(ref e) => {
            match e.log_level() {
                tracing::Level::ERROR => tracing::error!(error = %e, "failed"),
                tracing::Level::WARN => tracing::warn!(error = %e, "failed"),
                _ => tracing::debug!(error = %e, "failed"),
            }
            let body = ErrorBody {
                code: e.code().to_string(),
                status: e.status_code(),
            };
            Response::from_json(&body).map(|r| r.with_status(e.status_code()))
        }
    }
}
