use axum::http::StatusCode;
use axum::response::IntoResponse;
use tracing::instrument;

// Intentionally no action field — polled every 30s by Docker healthcheck,
// logging each call would flood prod logs.
#[instrument(skip_all)]
pub async fn health() -> impl IntoResponse {
    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn health_returns_200() {
        let response = health().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 64).await.unwrap();
        assert!(body.is_empty());
    }
}
