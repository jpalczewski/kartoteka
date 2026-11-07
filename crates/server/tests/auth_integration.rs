//! Integration tests for auth endpoints (C1 + C2 combined).

mod common;

use common::{TEST_PW, extract_session_cookie, json_body, register_and_login};

const WRONG_PW: &str = "test-wrong-pw";

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

/// Generate the current TOTP code for a base32 secret.
/// Mirrors domain::auth::make_totp (SHA-1, 6 digits, skew 1, 30s step).
fn generate_totp_code(secret_b32: &str) -> String {
    use totp_rs::{Algorithm, Builder, Secret};
    let secret = Secret::try_from_base32(secret_b32).unwrap();
    let totp = Builder::new()
        .with_algorithm(Algorithm::SHA1)
        .with_digits(6)
        .with_skew(1)
        .with_step_duration(30)
        .with_secret(secret)
        .build()
        .unwrap();
    totp.generate_current().to_string()
}

/// Session cookie helpers live in `common`.

// ── C1 tests (unchanged) ──────────────────────────────────────────────────

#[tokio::test]
async fn register_creates_first_user_as_admin() {
    let app = test_app().await;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": "admin@example.com", "password": "password123"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["role"], "admin");
}

#[tokio::test]
async fn login_wrong_password_returns_401() {
    let pool = kartoteka_db::test_helpers::test_pool().await;
    kartoteka_domain::auth::register(&pool, "user@example.com", TEST_PW, None)
        .await
        .unwrap();

    let session_store = SqliteStore::new(pool.clone());
    session_store.migrate().await.unwrap();
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);
    let backend = kartoteka_auth::KartotekaBackend::new(pool.clone());
    let auth_layer = axum_login::AuthManagerLayerBuilder::new(backend, session_layer).build();
    let app = kartoteka_server::test_router(
        pool,
        auth_layer,
        "test-secret-32-bytes-minimum!!!!".to_string(),
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": "user@example.com", "password": WRONG_PW}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn protected_route_without_auth_returns_401() {
    let app = test_app().await;
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/home")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ── C2 tests ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn totp_setup_requires_auth() {
    let app = test_app().await;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/totp/setup")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn totp_setup_returns_secret_and_url() {
    let app = test_app().await;

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": "user@test.com", "password": "pass123"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let login_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": "user@test.com", "password": "pass123"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_resp.status(), StatusCode::OK);
    let cookie = extract_session_cookie(&login_resp).expect("session cookie from login");

    let setup_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/totp/setup")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(setup_resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(setup_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["secret"].as_str().is_some_and(|s| !s.is_empty()));
    assert!(
        json["otpauth_url"]
            .as_str()
            .is_some_and(|u| u.starts_with("otpauth://totp/"))
    );
}

#[tokio::test]
async fn full_2fa_login_flow() {
    let app = test_app().await;

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": "totp@test.com", "password": "pass123"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let login_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": "totp@test.com", "password": "pass123"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let cookie = extract_session_cookie(&login_resp).expect("login session cookie");

    let setup_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/totp/setup")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let setup_body = axum::body::to_bytes(setup_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let setup_json: serde_json::Value = serde_json::from_slice(&setup_body).unwrap();
    let secret = setup_json["secret"].as_str().unwrap().to_string();

    let setup_code = generate_totp_code(&secret);
    let verify_setup_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/totp/verify")
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body(json_body(serde_json::json!({"code": setup_code})))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(verify_setup_resp.status(), StatusCode::OK);

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/logout")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let login2_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": "totp@test.com", "password": "pass123"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login2_resp.status(), StatusCode::OK);
    let pending_cookie = extract_session_cookie(&login2_resp).expect("pending session cookie");
    let login2_body = axum::body::to_bytes(login2_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let login2_json: serde_json::Value = serde_json::from_slice(&login2_body).unwrap();
    assert_eq!(login2_json["status"], "2fa_required");

    let login_code = generate_totp_code(&secret);
    let twofa_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/2fa")
                .header("cookie", &pending_cookie)
                .header("content-type", "application/json")
                .body(json_body(serde_json::json!({"code": login_code})))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(twofa_resp.status(), StatusCode::OK);
    let twofa_body = axum::body::to_bytes(twofa_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let twofa_json: serde_json::Value = serde_json::from_slice(&twofa_body).unwrap();
    assert_eq!(twofa_json["status"], "ok");
    assert!(twofa_json["user"]["id"].as_str().is_some());
}

#[tokio::test]
async fn verify_2fa_with_wrong_code_returns_401() {
    let app = test_app().await;

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": "wrong@test.com", "password": "pass123"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let login_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": "wrong@test.com", "password": "pass123"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let cookie = extract_session_cookie(&login_resp).unwrap();

    let setup_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/totp/setup")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let setup_body = axum::body::to_bytes(setup_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let setup_json: serde_json::Value = serde_json::from_slice(&setup_body).unwrap();
    let secret = setup_json["secret"].as_str().unwrap().to_string();

    let good_code = generate_totp_code(&secret);
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/totp/verify")
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body(json_body(serde_json::json!({"code": good_code})))
                .unwrap(),
        )
        .await
        .unwrap();

    let login2_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": "wrong@test.com", "password": "pass123"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let pending_cookie = extract_session_cookie(&login2_resp).unwrap();

    let bad_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/2fa")
                .header("cookie", &pending_cookie)
                .header("content-type", "application/json")
                .body(json_body(serde_json::json!({"code": "000000"})))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn delete_totp_disables_2fa() {
    let app = test_app().await;

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": "disable@test.com", "password": "pass123"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let login_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": "disable@test.com", "password": "pass123"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let cookie = extract_session_cookie(&login_resp).unwrap();

    let setup_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/totp/setup")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let setup_body = axum::body::to_bytes(setup_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let secret = serde_json::from_slice::<serde_json::Value>(&setup_body).unwrap()["secret"]
        .as_str()
        .unwrap()
        .to_string();
    let code = generate_totp_code(&secret);
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/totp/verify")
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body(json_body(serde_json::json!({"code": code})))
                .unwrap(),
        )
        .await
        .unwrap();

    let del_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/auth/totp")
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body(json_body(serde_json::json!({
                    "code": generate_totp_code(&secret),
                })))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_resp.status(), StatusCode::OK);

    let login2_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": "disable@test.com", "password": "pass123"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login2_resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(login2_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
}

/// Register a user, log in, set up and verify TOTP. Returns (app, cookie, secret).
async fn register_login_and_enable_totp(
    email: &str,
    password: &str,
) -> (axum::Router, String, String) {
    let app = test_app().await;
    let cookie = register_and_login(app.clone(), email, password).await;

    let setup_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/totp/setup")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let setup_body = axum::body::to_bytes(setup_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let secret = serde_json::from_slice::<serde_json::Value>(&setup_body).unwrap()["secret"]
        .as_str()
        .unwrap()
        .to_string();

    let verify_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/totp/verify")
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body(json_body(serde_json::json!({
                    "code": generate_totp_code(&secret),
                })))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(verify_resp.status(), StatusCode::OK);

    (app, cookie, secret)
}

#[tokio::test]
async fn delete_totp_with_wrong_code_returns_422() {
    let (app, cookie, _secret) =
        register_login_and_enable_totp("delete-wrong@test.com", "pass123").await;

    let del_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/auth/totp")
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body(json_body(serde_json::json!({"code": "000000"})))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // 2FA must still be required after a failed disable attempt.
    let login_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": "delete-wrong@test.com", "password": "pass123"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login_resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(login_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "2fa_required");
}

#[tokio::test]
async fn delete_totp_without_body_is_rejected() {
    let (app, cookie, _secret) =
        register_login_and_enable_totp("delete-nobody@test.com", "pass123").await;

    let del_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/auth/totp")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(del_resp.status().is_client_error());
}

// ── C3 tests: Bearer JWT tokens ──────────────────────────────────────────

#[tokio::test]
async fn create_token_and_use_bearer_on_api_route() {
    let app = test_app().await;
    let cookie = register_and_login(app.clone(), "user@example.com", TEST_PW).await;

    // Create a token via session
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/tokens")
                .header("content-type", "application/json")
                .header("cookie", &cookie)
                .body(json_body(serde_json::json!({"name": "My Token"})))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), 1024 * 64)
            .await
            .unwrap(),
    )
    .unwrap();
    let jwt = body["token"].as_str().unwrap().to_string();
    assert!(!jwt.is_empty());
    assert_eq!(body["scope"].as_str().unwrap(), "full");

    // Use the JWT as Bearer on a protected API route
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/settings")
                .header("authorization", format!("Bearer {jwt}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn revoked_bearer_token_returns_401() {
    let app = test_app().await;
    let cookie = register_and_login(app.clone(), "user@example.com", TEST_PW).await;

    // Create token
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/tokens")
                .header("content-type", "application/json")
                .header("cookie", &cookie)
                .body(json_body(serde_json::json!({"name": "Temp Token"})))
                .unwrap(),
        )
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), 1024 * 64)
            .await
            .unwrap(),
    )
    .unwrap();
    let jwt = body["token"].as_str().unwrap().to_string();
    let token_id = body["id"].as_str().unwrap().to_string();

    // Revoke it
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/auth/tokens/{token_id}"))
                .header("cookie", &cookie)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Revoked token must fail
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/settings")
                .header("authorization", format!("Bearer {jwt}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn list_tokens_returns_created_tokens() {
    let app = test_app().await;
    let cookie = register_and_login(app.clone(), "user@example.com", TEST_PW).await;

    // Initially empty
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/auth/tokens")
                .header("cookie", &cookie)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let list: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), 1024 * 64)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(list.as_array().unwrap().len(), 0);

    // Create two tokens
    for name in &["Token Alpha", "Token Beta"] {
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/tokens")
                    .header("content-type", "application/json")
                    .header("cookie", &cookie)
                    .body(json_body(serde_json::json!({"name": name})))
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/auth/tokens")
                .header("cookie", &cookie)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let list: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), 1024 * 64)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(list.as_array().unwrap().len(), 2);
    // token strings are NOT returned in list
    assert!(list[0].get("token").is_none());
}

#[tokio::test]
async fn create_token_without_auth_returns_401() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/tokens")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"name": "Unauthenticated Token"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn verify_2fa_locks_out_after_max_attempts() {
    // After MAX_2FA_ATTEMPTS bad codes the pending login must be cleared —
    // submitting a valid code afterwards should then fail with no_pending_2fa.
    let app = test_app().await;

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": "lock@test.com", "password": "pass123"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let login_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": "lock@test.com", "password": "pass123"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let cookie = extract_session_cookie(&login_resp).unwrap();

    let setup_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/totp/setup")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let secret = serde_json::from_slice::<serde_json::Value>(
        &axum::body::to_bytes(setup_resp.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap()["secret"]
        .as_str()
        .unwrap()
        .to_string();

    let code = generate_totp_code(&secret);
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/totp/verify")
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body(json_body(serde_json::json!({"code": code})))
                .unwrap(),
        )
        .await
        .unwrap();

    let login2_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": "lock@test.com", "password": "pass123"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let pending_cookie = extract_session_cookie(&login2_resp).unwrap();

    // 5 bad attempts → lockout
    for _ in 0..5 {
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/2fa")
                    .header("cookie", &pending_cookie)
                    .header("content-type", "application/json")
                    .body(json_body(serde_json::json!({"code": "000000"})))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    }

    // Even the correct code must now fail — pending_user_id was cleared.
    let good = generate_totp_code(&secret);
    let after = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/2fa")
                .header("cookie", &pending_cookie)
                .header("content-type", "application/json")
                .body(json_body(serde_json::json!({"code": good})))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(after.status(), StatusCode::UNAUTHORIZED);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(after.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["error"], "no_pending_2fa");
}

#[tokio::test]
async fn create_token_with_mcp_scope_is_rejected() {
    let app = test_app().await;
    let cookie = register_and_login(app.clone(), "mcp@test.com", TEST_PW).await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/tokens")
                .header("content-type", "application/json")
                .header("cookie", &cookie)
                .body(json_body(serde_json::json!({"name": "T", "scope": "mcp"})))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(resp.status(), StatusCode::CREATED);
    assert!(resp.status().is_client_error());
}

#[tokio::test]
async fn bearer_non_full_scope_rejected_on_api_routes() {
    let app = test_app().await;
    let cookie = register_and_login(app.clone(), "user@example.com", TEST_PW).await;

    // Create a readonly-scope token
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/tokens")
                .header("content-type", "application/json")
                .header("cookie", &cookie)
                .body(json_body(
                    serde_json::json!({"name": "Readonly Token", "scope": "readonly"}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(resp.into_body(), 1024 * 64)
            .await
            .unwrap(),
    )
    .unwrap();
    let jwt = body["token"].as_str().unwrap().to_string();

    // readonly-scope token must be rejected on /api/* routes (only "full" scope allowed)
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/settings")
                .header("authorization", format!("Bearer {jwt}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ── SSR auth guard (#218) ─────────────────────────────────────────────────

async fn get_page(
    app: axum::Router,
    uri: &str,
    cookie: Option<&str>,
) -> axum::http::Response<Body> {
    let mut req = Request::builder()
        .method("GET")
        .uri(uri)
        .header("accept", "text/html");
    if let Some(cookie) = cookie {
        req = req.header("cookie", cookie);
    }
    app.oneshot(req.body(Body::empty()).unwrap()).await.unwrap()
}

#[tokio::test]
async fn guarded_page_without_session_redirects_to_login_with_return_to() {
    let app = test_app().await;
    let response = get_page(app, "/lists/abc?tab=1", None).await;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(
        response.headers().get("location").unwrap(),
        "/login?return_to=%2Flists%2Fabc%3Ftab%3D1"
    );
}

#[tokio::test]
async fn guarded_page_with_session_renders() {
    let app = test_app().await;
    let cookie = register_and_login(app.clone(), "guard@example.com", TEST_PW).await;
    let response = get_page(app, "/all", Some(&cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get("location").is_none());
}

#[tokio::test]
async fn public_page_without_session_is_not_redirected() {
    let app = test_app().await;
    let response = get_page(app, "/login", None).await;
    assert_eq!(response.status(), StatusCode::OK);
}
