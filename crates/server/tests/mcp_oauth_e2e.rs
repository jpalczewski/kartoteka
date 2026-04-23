//! E2E smoke: DCR → direct code seeding → token exchange → Bearer on /mcp.
//! The browser-driven /oauth/authorize flow is unit-tested in the oauth crate.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration, Utc};
use kartoteka_auth::KartotekaBackend;
use kartoteka_db::test_helpers::{create_test_user, test_pool};
use kartoteka_mcp::McpI18n;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::SqliteStore;

/// Locates the workspace root (two dirs above this crate's manifest) and loads McpI18n from there.
fn load_mcp_i18n() -> Arc<McpI18n> {
    // CARGO_MANIFEST_DIR = .../crates/server — workspace root is two levels up.
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.join("..").join("..");
    let locales = workspace_root.join("locales");
    Arc::new(McpI18n::load_from(&locales))
}

fn build_app(pool: kartoteka_db::SqlitePool, mcp_i18n: Arc<McpI18n>) -> axum::Router {
    let session_store = SqliteStore::new(pool.clone());
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);
    let backend = KartotekaBackend::new(pool.clone());
    let auth_layer = axum_login::AuthManagerLayerBuilder::new(backend, session_layer).build();
    kartoteka_server::router(
        pool,
        auth_layer,
        "secret-at-least-32-chars-long-padded".into(),
        "http://localhost:3000".into(),
        leptos::config::LeptosOptions::builder()
            .output_name("kartoteka")
            .build(),
        mcp_i18n,
    )
}

#[tokio::test]
async fn dcr_then_token_then_mcp_initialize() {
    let pool = test_pool().await;
    // Migrate session store
    let session_store = SqliteStore::new(pool.clone());
    session_store.migrate().await.expect("session migrate");

    let mcp_i18n = load_mcp_i18n();
    let uid = create_test_user(&pool).await;
    let app = build_app(pool.clone(), mcp_i18n);

    // 1. DCR
    let dcr_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"client_name":"Test","redirect_uris":["http://x/cb"]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(dcr_res.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(dcr_res.into_body(), 8192)
        .await
        .unwrap();
    let dcr_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let client_id = dcr_json["client_id"].as_str().unwrap().to_string();

    // 2. Seed auth code directly (skipping browser authorize flow)
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    kartoteka_db::oauth::codes::insert(
        &pool,
        "code-e2e-test",
        &client_id,
        &uid,
        &challenge,
        "mcp",
        "http://x/cb",
        Utc::now() + Duration::minutes(5),
    )
    .await
    .unwrap();

    // 3. Token exchange
    let form = format!(
        "grant_type=authorization_code&code=code-e2e-test&redirect_uri=http%3A%2F%2Fx%2Fcb&client_id={client_id}&code_verifier={verifier}"
    );
    let tok_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/oauth/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tok_res.status(), StatusCode::OK);
    let tok_bytes = axum::body::to_bytes(tok_res.into_body(), 8192)
        .await
        .unwrap();
    let tok_json: serde_json::Value = serde_json::from_slice(&tok_bytes).unwrap();
    let access_token = tok_json["access_token"].as_str().unwrap().to_string();

    // 4. MCP initialize with Bearer
    let init_body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","clientInfo":{"name":"test","version":"0"},"capabilities":{}}}"#;
    let mcp_res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("authorization", format!("Bearer {access_token}"))
                .header("content-type", "application/json")
                .header("accept", "application/json, text/event-stream")
                .body(Body::from(init_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        mcp_res.status().is_success(),
        "MCP /initialize returned status: {}",
        mcp_res.status(),
    );
}
