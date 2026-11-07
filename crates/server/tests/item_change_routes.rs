//! REST contract for item mutations: `PUT /api/items/{id}` and
//! `POST /api/items/{id}/toggle` answer with `ItemChange` — the item's own
//! fields flattened (so existing clients keep working) plus an optional
//! `spawned` occurrence produced by a recurrence rule.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::SqliteStore;

const TEST_PW: &str = "test-placeholder-pw";

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

fn json_body(json: serde_json::Value) -> Body {
    Body::from(serde_json::to_vec(&json).unwrap())
}

/// Register the first user and log in, returning the session cookie.
async fn register_and_login(app: axum::Router, email: &str) -> String {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(json_body(
                    serde_json::json!({"email": email, "password": TEST_PW}),
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
                    serde_json::json!({"email": email, "password": TEST_PW}),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    resp.headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or("").to_string())
        .expect("session cookie after login")
}

/// Send one request with the session cookie and decode the JSON body.
async fn send_json(
    app: axum::Router,
    method: &str,
    uri: &str,
    cookie: &str,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("cookie", cookie);
    let body = match body {
        Some(json) => {
            builder = builder.header("content-type", "application/json");
            json_body(json)
        }
        None => Body::empty(),
    };
    let resp = app.oneshot(builder.body(body).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            panic!(
                "{method} {uri} -> {status}: {e}; body: {}",
                String::from_utf8_lossy(&bytes)
            )
        })
    };
    (status, json)
}

/// Create a list and return its id.
async fn create_list(app: axum::Router, cookie: &str) -> String {
    let (status, json) = send_json(
        app,
        "POST",
        "/api/lists",
        cookie,
        Some(serde_json::json!({
            "name": "Repeats",
            "list_type": "checklist",
            "features": ["checklist", "deadlines"],
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "list creation failed: {json}");
    json["id"].as_str().unwrap().to_string()
}

/// Create an item with a `2w` recurrence rule and a date to step from.
async fn create_recurring_item(app: axum::Router, cookie: &str, list_id: &str) -> String {
    let (status, json) = send_json(
        app,
        "POST",
        &format!("/api/lists/{list_id}/items"),
        cookie,
        Some(serde_json::json!({
            "title": "Water the plants",
            "deadline": "2026-11-07",
            "recurrence_rule": "2w",
            "recurrence_mode": "fixed",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "item creation failed: {json}");
    assert_eq!(json["recurrence_rule"], "2w");
    assert_eq!(json["recurrence_mode"], "fixed");
    json["id"].as_str().unwrap().to_string()
}

/// Completing a recurring item answers with the item plus the spawned occurrence.
#[tokio::test]
async fn toggle_of_recurring_item_returns_spawned_occurrence() {
    let app = test_app().await;
    let cookie = register_and_login(app.clone(), "toggle@example.com").await;
    let list_id = create_list(app.clone(), &cookie).await;
    let item_id = create_recurring_item(app.clone(), &cookie, &list_id).await;

    let (status, json) = send_json(
        app.clone(),
        "POST",
        &format!("/api/items/{item_id}/toggle"),
        &cookie,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "toggle failed: {json}");

    // Flattened item fields, so pre-recurrence clients still parse the response.
    assert_eq!(json["id"], item_id.as_str());
    assert_eq!(json["completed"], true);
    // The rule moved to the spawned occurrence.
    assert!(json.get("recurrence_rule").is_none_or(|v| v.is_null()));

    let spawned = json["spawned"]
        .as_object()
        .expect("toggle of a recurring item must spawn the next one");
    assert_ne!(spawned["id"], item_id.as_str());
    assert_eq!(spawned["title"], "Water the plants");
    assert_eq!(spawned["completed"], false);
    assert_eq!(spawned["deadline"], "2026-11-21");
    assert_eq!(spawned["recurrence_rule"], "2w");
    assert_eq!(spawned["recurrence_mode"], "fixed");
}

/// `PUT /api/items/{id}` also answers with `ItemChange`; a plain update has no `spawned`.
#[tokio::test]
async fn update_returns_item_change_without_spawned() {
    let app = test_app().await;
    let cookie = register_and_login(app.clone(), "update@example.com").await;
    let list_id = create_list(app.clone(), &cookie).await;
    let item_id = create_recurring_item(app.clone(), &cookie, &list_id).await;

    let (status, json) = send_json(
        app.clone(),
        "PUT",
        &format!("/api/items/{item_id}"),
        &cookie,
        Some(serde_json::json!({"title": "Water the ferns"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "update failed: {json}");
    assert_eq!(json["id"], item_id.as_str());
    assert_eq!(json["title"], "Water the ferns");
    assert_eq!(json["recurrence_rule"], "2w");
    assert!(
        json.get("spawned").is_none(),
        "an update that does not complete the item spawns nothing"
    );

    // Completing through `update` spawns too, on the same flattened contract.
    let (status, json) = send_json(
        app,
        "PUT",
        &format!("/api/items/{item_id}"),
        &cookie,
        Some(serde_json::json!({"completed": true})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "complete failed: {json}");
    assert_eq!(json["completed"], true);
    assert!(
        json["spawned"].is_object(),
        "completing a recurring item through PUT must spawn the next occurrence: {json}"
    );
}
