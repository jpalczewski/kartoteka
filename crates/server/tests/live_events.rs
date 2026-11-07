//! Integration tests for the live-update SSE endpoint `GET /api/events` (issue #357).

mod common;

use std::time::Duration;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use common::{TEST_PW, register_and_login};
use futures_util::StreamExt as _;
use kartoteka_db::changes;
use kartoteka_db::test_helpers::create_test_tag;
use kartoteka_db::{ChangeSignal, SqlitePool};
use kartoteka_domain::lists::CreateListRequest;
use kartoteka_server::ChangeHub;
use kartoteka_server::live::{MAX_STREAMS_PER_USER, UserChange, group_changes};
use kartoteka_shared::ChangeTopic;
use tokio::sync::mpsc;
use tower::ServiceExt;
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::SqliteStore;

const SECRET: &str = "test-secret-32-bytes-minimum!!!!";
/// Upper bound for one expected event; the pump coalesces for 50 ms.
const EVENT_TIMEOUT: Duration = Duration::from_secs(3);

/// A router whose change hub is wired to the pool's commit signal, plus the pool
/// the tests write through.
struct Fixture {
    app: Router,
    pool: SqlitePool,
}

async fn fixture() -> Fixture {
    // The hub only wakes up on writes through this pool, so the signal and the pool
    // must come from the same `create_pool_with_signal` call.
    let signal = ChangeSignal::default();
    let pool = kartoteka_db::create_pool_with_signal(":memory:", signal.clone())
        .await
        .expect("signal pool");
    kartoteka_db::run_migrations(&pool)
        .await
        .expect("migrations");

    let session_store = SqliteStore::new(pool.clone());
    session_store.migrate().await.expect("session migrate");
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);
    let backend = kartoteka_auth::KartotekaBackend::new(pool.clone());
    let auth_layer = axum_login::AuthManagerLayerBuilder::new(backend, session_layer).build();

    let change_hub = ChangeHub::start(pool.clone(), signal);
    let app = kartoteka_server::test_router_with_change_hub(
        pool.clone(),
        change_hub,
        auth_layer,
        SECRET.to_string(),
        kartoteka_server::TrustedProxies::none(),
    );
    Fixture { app, pool }
}

/// One parsed SSE event from the response body.
#[derive(Debug, Clone)]
struct SseEvent {
    event: String,
    id: Option<i64>,
    data: String,
}

impl SseEvent {
    /// The `topics` array of a `change` event's JSON payload.
    fn topics(&self) -> Vec<ChangeTopic> {
        let json: serde_json::Value = serde_json::from_str(&self.data).expect("event data is JSON");
        serde_json::from_value(json["topics"].clone()).expect("topics are ChangeTopics")
    }
}

async fn connect(
    app: &Router,
    cookie: &str,
    last_event_id: Option<i64>,
) -> axum::http::Response<Body> {
    let mut request = Request::builder()
        .method("GET")
        .uri("/api/events")
        .header("cookie", cookie);
    if let Some(id) = last_event_id {
        request = request.header("last-event-id", id.to_string());
    }
    app.clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

/// Reads the response body in the background and parses events as they arrive, so a
/// test can wait for one with a timeout instead of driving the stream itself.
fn read_events(body: Body) -> mpsc::UnboundedReceiver<SseEvent> {
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        let mut chunks = body.into_data_stream();
        let mut buf = String::new();
        while let Some(chunk) = chunks.next().await {
            let Ok(chunk) = chunk else { break };
            buf.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(end) = buf.find("\n\n") {
                let block: String = buf.drain(..end + 2).collect();
                if let Some(event) = parse_event(&block)
                    && tx.send(event).is_err()
                {
                    return;
                }
            }
        }
    });
    rx
}

fn parse_event(block: &str) -> Option<SseEvent> {
    let mut event = "message".to_string();
    let mut id = None;
    let mut data = String::new();
    for line in block.lines() {
        if let Some(rest) = line.strip_prefix("event: ") {
            event = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("id: ") {
            id = rest.trim().parse().ok();
        } else if let Some(rest) = line.strip_prefix("data: ") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest);
        }
    }
    // Keep-alive comments (`:`) carry no fields.
    if event == "message" && id.is_none() && data.is_empty() {
        return None;
    }
    Some(SseEvent { event, id, data })
}

async fn next_event(events: &mut mpsc::UnboundedReceiver<SseEvent>) -> SseEvent {
    tokio::time::timeout(EVENT_TIMEOUT, events.recv())
        .await
        .expect("an event within the timeout")
        .expect("the stream is still open")
}

async fn user_id(pool: &SqlitePool, email: &str) -> String {
    sqlx::query_scalar::<_, String>("SELECT id FROM users WHERE email = ?")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("user id")
}

/// Creates a list through the domain layer, i.e. a real write path behind a trigger.
async fn create_list(pool: &SqlitePool, user_id: &str, name: &str) -> String {
    let request = CreateListRequest {
        name: name.to_string(),
        list_type: None,
        icon: None,
        color: None,
        description: None,
        container_id: None,
        parent_list_id: None,
        features: Vec::new(),
    };
    kartoteka_domain::lists::create(pool, user_id, &request)
        .await
        .expect("create list")
        .id
}

// ── auth and the happy path ──────────────────────────────────────────────

#[tokio::test]
async fn unauthenticated_stream_is_rejected() {
    let f = fixture().await;
    let response = f
        .app
        .oneshot(
            Request::builder()
                .uri("/api/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn the_first_event_is_ready_with_an_id() {
    let f = fixture().await;
    let cookie = register_and_login(f.app.clone(), "ready@example.com", TEST_PW).await;

    let response = connect(&f.app, &cookie, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("text/event-stream")
    );
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok()),
        Some("no-cache")
    );
    assert_eq!(
        response
            .headers()
            .get("x-accel-buffering")
            .and_then(|v| v.to_str().ok()),
        Some("no"),
        "nginx must not buffer the stream"
    );

    let mut events = read_events(response.into_body());
    let first = next_event(&mut events).await;
    assert_eq!(first.event, "ready");
    assert_eq!(first.id, Some(changes::max_id(&f.pool).await.unwrap()));
    assert_eq!(first.data, "{}");
}

#[tokio::test]
async fn a_write_by_the_same_user_is_streamed() {
    let f = fixture().await;
    let cookie = register_and_login(f.app.clone(), "writer@example.com", TEST_PW).await;
    let user = user_id(&f.pool, "writer@example.com").await;

    let mut events = read_events(connect(&f.app, &cookie, None).await.into_body());
    assert_eq!(next_event(&mut events).await.event, "ready");

    create_list(&f.pool, &user, "Groceries").await;

    let change = next_event(&mut events).await;
    assert_eq!(change.event, "change");
    assert!(change.id.is_some(), "every event carries an id");
    assert_eq!(change.topics(), vec![ChangeTopic::Lists]);
}

#[tokio::test]
async fn a_write_by_another_user_is_not_streamed() {
    let f = fixture().await;
    let cookie_a = register_and_login(f.app.clone(), "alice@example.com", TEST_PW).await;
    register_and_login(f.app.clone(), "bob@example.com", TEST_PW).await;
    let alice = user_id(&f.pool, "alice@example.com").await;
    let bob = user_id(&f.pool, "bob@example.com").await;

    let mut events = read_events(connect(&f.app, &cookie_a, None).await.into_body());
    assert_eq!(next_event(&mut events).await.event, "ready");

    // Bob writes a tag, then Alice writes a list: both rows reach the pump, but Alice's
    // stream may only ever report her own change.
    create_test_tag(&f.pool, &bob, "bobs-tag").await;
    create_list(&f.pool, &alice, "Alices list").await;

    let change = next_event(&mut events).await;
    assert_eq!(change.event, "change");
    assert_eq!(
        change.topics(),
        vec![ChangeTopic::Lists],
        "bob's tags must not reach alice"
    );
}

// ── replay ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_valid_last_event_id_replays_missed_topics() {
    let f = fixture().await;
    let cookie = register_and_login(f.app.clone(), "replay@example.com", TEST_PW).await;
    let user = user_id(&f.pool, "replay@example.com").await;

    create_list(&f.pool, &user, "First").await;
    create_list(&f.pool, &user, "Second").await;
    let rows = changes::fetch_after(&f.pool, 0, 100).await.unwrap();
    let (first, last) = (rows[0].id, rows.last().unwrap().id);
    assert!(first < last);

    // Reconnect as if the client had only seen the first row.
    let mut events = read_events(connect(&f.app, &cookie, Some(first)).await.into_body());
    let replay = next_event(&mut events).await;
    assert_eq!(replay.event, "change", "no `ready` when replaying");
    assert_eq!(replay.id, Some(last));
    assert_eq!(replay.topics(), vec![ChangeTopic::Lists]);
}

#[tokio::test]
async fn a_last_event_id_below_the_pruned_range_gets_a_resync() {
    let f = fixture().await;
    let cookie = register_and_login(f.app.clone(), "stale@example.com", TEST_PW).await;
    let user = user_id(&f.pool, "stale@example.com").await;

    for name in ["One", "Two", "Three"] {
        create_list(&f.pool, &user, name).await;
    }
    let rows = changes::fetch_after(&f.pool, 0, 100).await.unwrap();
    let newest = rows.last().unwrap().id;
    // Simulate the pump's prune: everything below the newest row is gone.
    sqlx::query("DELETE FROM change_log WHERE id < ?")
        .bind(newest)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(changes::min_id(&f.pool).await.unwrap(), Some(newest));

    // A reconnect from inside the pruned window cannot be replayed.
    let stale = newest - 2;
    let mut events = read_events(connect(&f.app, &cookie, Some(stale)).await.into_body());
    let event = next_event(&mut events).await;
    assert_eq!(event.event, "resync");
    assert_eq!(event.id, Some(changes::max_id(&f.pool).await.unwrap()));
    assert_eq!(event.data, "{}");
}

// ── per-user stream cap ──────────────────────────────────────────────────

#[tokio::test]
async fn the_ninth_concurrent_stream_is_throttled() {
    let f = fixture().await;
    let cookie = register_and_login(f.app.clone(), "many@example.com", TEST_PW).await;
    let other = register_and_login(f.app.clone(), "other@example.com", TEST_PW).await;

    // Hold every response: its body owns the stream, and the stream owns the slot.
    let mut open = Vec::new();
    for _ in 0..MAX_STREAMS_PER_USER {
        let response = connect(&f.app, &cookie, None).await;
        assert_eq!(response.status(), StatusCode::OK);
        open.push(response);
    }

    let ninth = connect(&f.app, &cookie, None).await;
    assert_eq!(ninth.status(), StatusCode::TOO_MANY_REQUESTS);

    let other_user = connect(&f.app, &other, None).await;
    assert_eq!(other_user.status(), StatusCode::OK, "the cap is per user");

    // Dropping a response body releases the slot.
    open.pop();
    assert_eq!(
        connect(&f.app, &cookie, None).await.status(),
        StatusCode::OK
    );
}

// ── pump grouping ────────────────────────────────────────────────────────

#[test]
fn grouping_batches_per_user_and_dedupes_topics() {
    let change = |id: i64, user: &str, topic: &str| kartoteka_db::changes::ChangeRow {
        id,
        user_id: user.to_string(),
        topic: topic.to_string(),
    };
    let rows = vec![
        change(1, "u1", "lists"),
        change(2, "u2", "items"),
        change(3, "u1", "lists"),
        change(4, "u1", "tags"),
        change(5, "u2", "not_a_topic"),
    ];

    let grouped: Vec<UserChange> = group_changes(rows);
    assert_eq!(grouped.len(), 2, "one UserChange per user");

    let u1 = grouped.iter().find(|c| c.user_id == "u1").unwrap();
    assert_eq!(u1.last_id, 4);
    assert_eq!(u1.topics, vec![ChangeTopic::Lists, ChangeTopic::Tags]);

    let u2 = grouped.iter().find(|c| c.user_id == "u2").unwrap();
    assert_eq!(u2.last_id, 2);
    assert_eq!(
        u2.topics,
        vec![ChangeTopic::Items],
        "unknown topics skipped"
    );
}
