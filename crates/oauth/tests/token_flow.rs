use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration, Utc};
use kartoteka_db::test_helpers::{create_test_user, test_pool};
use kartoteka_oauth::{OAuthState, routes};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

fn app(pool: kartoteka_db::SqlitePool) -> axum::Router {
    routes().with_state(OAuthState {
        pool,
        signing_secret: "secret-at-least-32-chars-long-padded".into(),
        public_base_url: "http://localhost:3000".into(),
    })
}

#[tokio::test]
async fn auth_code_with_pkce_round_trip() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    kartoteka_db::oauth::clients::create(&pool, "c1", "Claude", r#"["http://x/cb"]"#)
        .await
        .unwrap();

    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let expires = Utc::now() + Duration::minutes(5);
    kartoteka_db::oauth::codes::insert(
        &pool,
        "code-1",
        "c1",
        &uid,
        &challenge,
        "mcp",
        "http://x/cb",
        expires,
    )
    .await
    .unwrap();

    let form = format!(
        "grant_type=authorization_code&code=code-1&redirect_uri=http%3A%2F%2Fx%2Fcb&client_id=c1&code_verifier={verifier}"
    );
    let res = app(pool.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 8192).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["token_type"], "Bearer");
    assert_eq!(v["scope"], "mcp");
    assert!(v["access_token"].as_str().unwrap().contains('.'));
    assert!(v["refresh_token"].as_str().unwrap().len() > 20);
}

#[tokio::test]
async fn code_replay_rejected() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    kartoteka_db::oauth::clients::create(&pool, "c1", "X", r#"["http://x/cb"]"#)
        .await
        .unwrap();
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    kartoteka_db::oauth::codes::insert(
        &pool,
        "code-2",
        "c1",
        &uid,
        &challenge,
        "mcp",
        "http://x/cb",
        Utc::now() + Duration::minutes(5),
    )
    .await
    .unwrap();

    let form = format!(
        "grant_type=authorization_code&code=code-2&redirect_uri=http%3A%2F%2Fx%2Fcb&client_id=c1&code_verifier={verifier}"
    );
    let a = app(pool.clone());
    let r1 = a
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r1.status(), StatusCode::OK);
    let r2 = a
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r2.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn refresh_rotation_invalidates_old() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    kartoteka_db::oauth::clients::create(&pool, "c1", "X", r#"["http://x/cb"]"#)
        .await
        .unwrap();

    let old = "old-refresh-token-12345-padding-for-bytes";
    let old_hash = URL_SAFE_NO_PAD.encode(Sha256::digest(old.as_bytes()));
    kartoteka_db::oauth::refresh::insert(
        &pool,
        &old_hash,
        "c1",
        &uid,
        "mcp",
        Utc::now() + Duration::days(30),
    )
    .await
    .unwrap();

    let form = format!("grant_type=refresh_token&refresh_token={old}&client_id=c1");
    let a = app(pool.clone());
    let r1 = a
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r1.status(), StatusCode::OK);

    let r2 = a
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r2.status(), StatusCode::BAD_REQUEST);
}
