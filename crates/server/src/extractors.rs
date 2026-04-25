use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};

/// User identity extracted from request extensions.
/// Populated by `auth::require_auth` middleware for all protected routes.
#[derive(Clone)]
pub struct UserId(pub String);

impl<S: Send + Sync> FromRequestParts<S> for UserId {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<UserId>()
            .cloned()
            .ok_or((StatusCode::UNAUTHORIZED, "not authenticated"))
    }
}
