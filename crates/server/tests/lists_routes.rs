//! Regression tests for list endpoint routing (no double `/lists` prefix).

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::SqliteStore;

async fn test_app() -> axum::Router {
    let pool = kartoteka_db::test_helpers::test_pool().await;
    let session_store = SqliteStore::new(pool.clone());
    session_store.migrate().await.unwrap();
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);
    let backend = kartoteka_auth::KartotekaBackend::new(pool.clone());
    let auth_layer = axum_login::AuthManagerLayerBuilder::new(backend, session_layer).build();
    kartoteka_server::test_router(
        pool,
        auth_layer,
        "test-secret-32-bytes-minimum!!!!".to_string(),
    )
}

/// Unauthenticated request to a real list route should hit auth middleware
/// (401), not the fallback 404. This guards against the double `/lists` prefix
/// regression — before the fix every list route resolved to 404.
#[tokio::test]
async fn list_all_route_is_registered() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/lists")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "GET /api/lists must be routed (expected auth failure, got 404)"
    );
}

#[tokio::test]
async fn list_archived_route_is_registered() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/lists/archived")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(resp.status(), StatusCode::NOT_FOUND);
}
