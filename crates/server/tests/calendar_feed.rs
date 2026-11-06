//! Integration tests for the public iCal feed endpoint `GET /cal/{token}/feed.ics`.

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use kartoteka_db::test_helpers::{create_test_user, test_pool};
use kartoteka_domain::calendar::{CalendarTokenCreated, create_token, revoke_token};
use sqlx::SqlitePool;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::SqliteStore;

/// Far future, so the item always falls inside the feed's date window.
const FUTURE: &str = "2999-01-01";
/// Client IP used by default; the rate limiter keys on it.
const CLIENT_IP: &str = "203.0.113.7";
/// Ids are plain TEXT in this schema, so a counter is enough here.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn next_id(prefix: &str) -> String {
    format!("{prefix}-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed))
}

/// A router plus the data needed to request its feed.
struct Fixture {
    app: Router,
    pool: SqlitePool,
    token: CalendarTokenCreated,
}

async fn fixture() -> Fixture {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let list_id = create_list(&pool, &user_id, "Groceries").await;
    create_item(&pool, &list_id, "Buy milk").await;
    let token = create_token(&pool, &user_id, "My feed", None)
        .await
        .expect("create token");
    let app = test_app(pool.clone()).await;
    Fixture { app, pool, token }
}

/// Same construction as `lists_routes.rs`, on the caller's pool so the test can
/// seed data the router will read.
async fn test_app(pool: SqlitePool) -> Router {
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

async fn create_list(pool: &SqlitePool, user_id: &str, name: &str) -> String {
    let list_id = next_id("list");
    sqlx::query("INSERT INTO lists (id, user_id, name) VALUES (?, ?, ?)")
        .bind(&list_id)
        .bind(user_id)
        .bind(name)
        .execute(pool)
        .await
        .unwrap();
    list_id
}

async fn create_item(pool: &SqlitePool, list_id: &str, title: &str) -> String {
    let item_id = next_id("item");
    sqlx::query("INSERT INTO items (id, list_id, title, start_date) VALUES (?, ?, ?, ?)")
        .bind(&item_id)
        .bind(list_id)
        .bind(title)
        .bind(FUTURE)
        .execute(pool)
        .await
        .unwrap();
    item_id
}

fn feed_uri(token: &str) -> String {
    format!("/cal/{token}/feed.ics")
}

/// GET the feed from `ip`, optionally with extra headers.
fn request(uri: &str, ip: &str, headers: &[(header::HeaderName, &str)]) -> Request<Body> {
    let mut builder = Request::builder()
        .method("GET")
        .uri(uri)
        .header("x-forwarded-for", ip);
    for (name, value) in headers {
        builder = builder.header(name, *value);
    }
    builder.body(Body::empty()).unwrap()
}

async fn send(app: Router, req: Request<Body>) -> axum::http::Response<axum::body::Body> {
    app.oneshot(req).await.unwrap()
}

#[tokio::test]
async fn returns_calendar_with_etag_and_item_title() {
    let Fixture { app, token, .. } = fixture().await;
    let resp = send(app, request(&feed_uri(&token.token), CLIENT_IP, &[])).await;

    assert_eq!(resp.status(), StatusCode::OK);
    let headers = resp.headers();
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "text/calendar; charset=utf-8"
    );
    assert!(
        headers.get(header::ETAG).is_some(),
        "feed must carry a strong ETag"
    );
    assert_eq!(
        headers.get(header::CACHE_CONTROL).unwrap(),
        "private, max-age=900"
    );
    assert_eq!(
        headers.get(header::CONTENT_DISPOSITION).unwrap(),
        "inline; filename=\"kartoteka.ics\""
    );
    assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");

    let body = to_bytes(resp.into_body(), 1_000_000).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        body.starts_with("BEGIN:VCALENDAR"),
        "unexpected body: {body}"
    );
    assert!(body.contains("Buy milk"), "item missing from feed: {body}");
}

/// The `feed.ics` segment contains a dot — the dotted-segment middleware must
/// let it through instead of answering 404.
#[tokio::test]
async fn dotted_segment_middleware_allows_feed_path() {
    let Fixture { app, token, .. } = fixture().await;
    let resp = send(app, request(&feed_uri(&token.token), CLIENT_IP, &[])).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "dotted-segment middleware must not reject /cal/{{token}}/feed.ics"
    );
}

#[tokio::test]
async fn returns_304_for_matching_if_none_match() {
    let Fixture { app, token, .. } = fixture().await;
    let uri = feed_uri(&token.token);
    let resp = send(app.clone(), request(&uri, CLIENT_IP, &[])).await;
    let etag = resp
        .headers()
        .get(header::ETAG)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // Weak form, as some clients send it.
    for candidate in [
        etag.clone(),
        format!("W/{etag}"),
        format!("\"nope\", {etag}"),
    ] {
        let resp = send(
            app.clone(),
            request(&uri, CLIENT_IP, &[(header::IF_NONE_MATCH, &candidate)]),
        )
        .await;
        assert_eq!(
            resp.status(),
            StatusCode::NOT_MODIFIED,
            "If-None-Match {candidate} should match"
        );
        assert_eq!(resp.headers().get(header::ETAG).unwrap(), etag.as_str());
        assert_eq!(
            resp.headers().get(header::CACHE_CONTROL).unwrap(),
            "private, max-age=900"
        );
        let body = to_bytes(resp.into_body(), 1_000_000).await.unwrap();
        assert!(body.is_empty(), "304 must have an empty body");
    }
}

#[tokio::test]
async fn stale_if_none_match_returns_full_feed() {
    let Fixture { app, token, .. } = fixture().await;
    let resp = send(
        app,
        request(
            &feed_uri(&token.token),
            CLIENT_IP,
            &[(header::IF_NONE_MATCH, "\"stale\"")],
        ),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn unknown_token_returns_404() {
    let Fixture { app, .. } = fixture().await;
    let unknown = "A".repeat(43);
    let resp = send(app, request(&feed_uri(&unknown), CLIENT_IP, &[])).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = to_bytes(resp.into_body(), 1_000_000).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        !body.contains("calendar"),
        "404 body must not hint at the reason: {body}"
    );
}

#[tokio::test]
async fn revoked_token_returns_404() {
    let Fixture {
        app, pool, token, ..
    } = fixture().await;
    // Revoke needs the owning user id, which the token row carries.
    let user_id: String = sqlx::query_scalar("SELECT user_id FROM calendar_tokens WHERE id = ?")
        .bind(&token.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    revoke_token(&pool, &user_id, &token.id).await.unwrap();

    let resp = send(app, request(&feed_uri(&token.token), CLIENT_IP, &[])).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn rate_limits_per_client_ip() {
    let Fixture { app, token, .. } = fixture().await;
    let uri = feed_uri(&token.token);
    // Burst is 20, so the first 20 requests from one IP pass.
    for i in 0..20 {
        let resp = send(app.clone(), request(&uri, "198.51.100.10", &[])).await;
        assert_eq!(resp.status(), StatusCode::OK, "request {i} should pass");
    }

    let resp = send(app.clone(), request(&uri, "198.51.100.10", &[])).await;
    assert_eq!(
        resp.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "the 21st request from one IP must be throttled"
    );

    // Another client keeps its own quota.
    let resp = send(app, request(&uri, "198.51.100.11", &[])).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "a different IP must not be affected"
    );
}

#[tokio::test]
async fn no_session_cookie_required() {
    let Fixture { app, token, .. } = fixture().await;
    let req = Request::builder()
        .method("GET")
        .uri(feed_uri(&token.token))
        .header("x-forwarded-for", CLIENT_IP)
        .body(Body::empty())
        .unwrap();
    assert!(req.headers().get(header::COOKIE).is_none());

    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn head_request_returns_headers_without_body() {
    let Fixture { app, token, .. } = fixture().await;
    let req = Request::builder()
        .method("HEAD")
        .uri(feed_uri(&token.token))
        .header("x-forwarded-for", CLIENT_IP)
        .body(Body::empty())
        .unwrap();
    let resp = send(app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get(header::CONTENT_TYPE).unwrap(),
        "text/calendar; charset=utf-8"
    );
}
