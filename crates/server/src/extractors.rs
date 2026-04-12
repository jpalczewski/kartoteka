use axum::{extract::FromRequestParts, http::{request::Parts, StatusCode}};

/// Placeholder user identity extractor.
/// Reads `X-User-Id` header. Replaced by real auth middleware in C1.
pub struct UserId(pub String);

impl<S: Send + Sync> FromRequestParts<S> for UserId {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .headers
            .get("x-user-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| UserId(s.to_string()))
            .ok_or((StatusCode::UNAUTHORIZED, "X-User-Id header required"))
    }
}
