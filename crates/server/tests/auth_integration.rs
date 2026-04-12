//! Integration tests for auth endpoints.

use axum::{body::Body, http::{Request, StatusCode}};
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
    kartoteka_server::router(pool, auth_layer)
}

fn json_body(json: serde_json::Value) -> Body {
    Body::from(serde_json::to_vec(&json).unwrap())
}

#[tokio::test]
async fn register_creates_first_user_as_admin() {
    let app = test_app().await;
    let response = app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/auth/register")
            .header("content-type", "application/json")
            .body(json_body(serde_json::json!({"email": "admin@example.com", "password": "password123"})))
            .unwrap(),
    ).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["role"], "admin");
}

#[tokio::test]
async fn login_wrong_password_returns_401() {
    let pool = kartoteka_db::test_helpers::test_pool().await;
    // Register first
    kartoteka_domain::auth::register(&pool, "user@example.com", "correct", None).await.unwrap();

    let session_store = SqliteStore::new(pool.clone());
    session_store.migrate().await.unwrap();
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);
    let backend = kartoteka_auth::KartotekaBackend::new(pool.clone());
    let auth_layer = axum_login::AuthManagerLayerBuilder::new(backend, session_layer).build();
    let app = kartoteka_server::router(pool, auth_layer);

    let response = app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/auth/login")
            .header("content-type", "application/json")
            .body(json_body(serde_json::json!({"email": "user@example.com", "password": "wrong"})))
            .unwrap(),
    ).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn protected_route_without_auth_returns_401() {
    let app = test_app().await;
    let response = app.oneshot(
        Request::builder()
            .method("GET")
            .uri("/api/home")
            .body(Body::empty())
            .unwrap(),
    ).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
