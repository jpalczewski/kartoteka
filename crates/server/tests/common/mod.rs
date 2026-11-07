//! Helpers shared by the server integration tests.
//!
//! `tests/common/mod.rs` is a subdirectory module, so cargo does not build it as a
//! test binary of its own. Not every test file uses every helper.
#![allow(dead_code)]

use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

pub const TEST_PW: &str = "test-placeholder-pw";

pub fn json_body(json: serde_json::Value) -> Body {
    Body::from(serde_json::to_vec(&json).unwrap())
}

/// Extract the session cookie value from a Set-Cookie header (just name=value, no attributes).
pub fn extract_session_cookie(response: &axum::http::Response<Body>) -> Option<String> {
    response
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or("").to_string())
}

/// Register a user and return the session cookie after login.
pub async fn register_and_login(app: axum::Router, email: &str, password: &str) -> String {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": email, "password": password}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": email, "password": password}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    extract_session_cookie(&resp).expect("session cookie after login")
}
