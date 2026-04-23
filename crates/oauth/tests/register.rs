use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use kartoteka_db::test_helpers::test_pool;
use kartoteka_oauth::{OAuthState, routes};
use tower::ServiceExt;

#[tokio::test]
async fn dcr_creates_client() {
    let pool = test_pool().await;
    let state = OAuthState {
        pool: pool.clone(),
        signing_secret: "test-secret-32-chars-padding-here-pls".into(),
        public_base_url: "http://localhost:3000".into(),
    };
    let app = routes().with_state(state);

    let req = Request::builder()
        .method("POST")
        .uri("/register")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"client_name":"Claude","redirect_uris":["http://localhost:33418/cb"]}"#,
        ))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 8192).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(v["client_id"].as_str().unwrap().len() > 10);
}

#[tokio::test]
async fn dcr_rejects_empty_redirect_uris() {
    let pool = test_pool().await;
    let state = OAuthState {
        pool,
        signing_secret: "x".repeat(32),
        public_base_url: "http://localhost:3000".into(),
    };
    let app = routes().with_state(state);
    let req = Request::builder()
        .method("POST")
        .uri("/register")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"client_name":"X","redirect_uris":[]}"#))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}
