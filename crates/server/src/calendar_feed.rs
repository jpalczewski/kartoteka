//! Public iCal feed: `GET /cal/{token}/feed.ics`.
//!
//! Served outside `/api` and without any session or cookie — the token in the
//! path is the only credential, so it must never reach a log line or a span
//! field. The route is mounted on the base router and rate limited per client
//! IP, because an unauthenticated endpoint is an easy amplification target.

use std::time::Duration;

use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{
        HeaderMap, HeaderName, HeaderValue, StatusCode,
        header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE, ETAG, IF_NONE_MATCH},
    },
    response::{IntoResponse, Response},
    routing::get,
};
use kartoteka_db::SqlitePool;
use kartoteka_domain::calendar::{Feed, feed_for_token};
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor,
};

use crate::AppState;

/// Requests a single client IP may issue in a burst before throttling.
const BURST_SIZE: u32 = 20;
/// Seconds until one request of the burst quota is replenished.
const REPLENISH_SECONDS: u64 = 3;
/// How often idle per-IP limiter state is dropped.
const CLEANUP_INTERVAL: Duration = Duration::from_secs(60);

const CALENDAR_CONTENT_TYPE: &str = "text/calendar; charset=utf-8";
/// Calendar clients poll on their own schedule; 15 minutes of private caching is plenty.
const CACHE_CONTROL_PRIVATE: &str = "private, max-age=900";
const CONTENT_DISPOSITION_INLINE: &str = "inline; filename=\"kartoteka.ics\"";
const X_CONTENT_TYPE_OPTIONS: &str = "x-content-type-options";
const NOSNIFF: &str = "nosniff";
/// Deliberately information-free bodies: unknown, revoked and malformed tokens
/// must all look the same.
const NOT_FOUND_BODY: &str = "Not Found";
const INTERNAL_ERROR_BODY: &str = "Internal Server Error";

/// `/cal/{token}/feed.ics` — rate limited per client IP, no auth middleware.
///
/// Burst of [`BURST_SIZE`] requests per IP, then one more every
/// [`REPLENISH_SECONDS`]. The extractor prefers `X-Forwarded-For` /
/// `X-Real-IP` / `Forwarded` (the app runs behind a reverse proxy) and falls
/// back to the peer address from `ConnectInfo`, which is why `main` serves with
/// `into_make_service_with_connect_info::<SocketAddr>()`. Those headers are only
/// trustworthy when the reverse proxy overwrites client-supplied ones.
pub fn router() -> Router<AppState> {
    let config = GovernorConfigBuilder::default()
        .per_second(REPLENISH_SECONDS)
        .burst_size(BURST_SIZE)
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .expect("governor config: burst size and period are non-zero");

    // Drop idle per-IP limiter state so the keyed map cannot grow forever. The
    // task lives on the current runtime, so integration tests get one cheap
    // task per router (cancelled on runtime shutdown) and non-async callers are
    // simply skipped.
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        let limiter = config.limiter().clone();
        handle.spawn(async move {
            let mut ticker = tokio::time::interval(CLEANUP_INTERVAL);
            // The first tick of a tokio interval completes immediately.
            ticker.tick().await;
            loop {
                ticker.tick().await;
                limiter.retain_recent();
            }
        });
    }

    Router::new()
        .route("/{token}/feed.ics", get(feed))
        .layer(GovernorLayer::new(config))
}

/// Render the feed for a subscribe token.
///
/// Unknown, revoked and malformed tokens all yield the same bare 404. The token
/// argument is skipped by `skip_all`, so it appears in no span field.
#[tracing::instrument(skip_all, fields(action = "get_calendar_feed"))]
pub async fn feed(
    State(pool): State<SqlitePool>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Response {
    match feed_for_token(&pool, &token).await {
        Ok(Some(feed)) => {
            if if_none_match_matches(&headers, &feed.etag) {
                not_modified_response(&feed.etag)
            } else {
                feed_response(feed)
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, NOT_FOUND_BODY).into_response(),
        Err(error) => {
            tracing::error!(%error, "calendar feed generation failed");
            (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL_ERROR_BODY).into_response()
        }
    }
}

/// 200 with the feed body and its caching headers.
fn feed_response(feed: Feed) -> Response {
    let Ok(etag) = HeaderValue::from_str(&feed.etag) else {
        tracing::error!("calendar feed rendered an invalid ETag");
        return (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL_ERROR_BODY).into_response();
    };
    let mut response = Response::new(Body::from(feed.body));
    let headers = response.headers_mut();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(CALENDAR_CONTENT_TYPE),
    );
    headers.insert(ETAG, etag);
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static(CACHE_CONTROL_PRIVATE),
    );
    headers.insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_static(CONTENT_DISPOSITION_INLINE),
    );
    headers.insert(
        HeaderName::from_static(X_CONTENT_TYPE_OPTIONS),
        HeaderValue::from_static(NOSNIFF),
    );
    response
}

/// 304 carrying the validators, so clients keep their cached copy.
fn not_modified_response(etag: &str) -> Response {
    let Ok(etag) = HeaderValue::from_str(etag) else {
        return (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL_ERROR_BODY).into_response();
    };
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_MODIFIED;
    let headers = response.headers_mut();
    headers.insert(ETAG, etag);
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static(CACHE_CONTROL_PRIVATE),
    );
    response
}

/// `If-None-Match` evaluation: a comma-separated list, `*`, and weak
/// (`W/"…"`) candidates all count as a match for our strong ETag.
fn if_none_match_matches(headers: &HeaderMap, etag: &str) -> bool {
    let Some(value) = headers
        .get(IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    let etag = opaque_tag(etag);
    value
        .split(',')
        .map(str::trim)
        .any(|candidate| candidate == "*" || opaque_tag(candidate) == etag)
}

/// The tag without its optional `W/` prefix and surrounding quotes, so
/// `W/"abc"`, `"abc"` and `abc` all normalise to `abc`.
fn opaque_tag(raw: &str) -> &str {
    let raw = raw.trim();
    let raw = raw.strip_prefix("W/").unwrap_or(raw).trim();
    raw.strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_identical_etag() {
        let mut headers = HeaderMap::new();
        headers.insert(IF_NONE_MATCH, HeaderValue::from_static("\"abc123\""));
        assert!(if_none_match_matches(&headers, "\"abc123\""));
    }

    #[test]
    fn matches_weak_and_listed_candidates() {
        let mut headers = HeaderMap::new();
        headers.insert(
            IF_NONE_MATCH,
            HeaderValue::from_static("\"other\", W/\"abc123\""),
        );
        assert!(if_none_match_matches(&headers, "\"abc123\""));
    }

    #[test]
    fn matches_wildcard() {
        let mut headers = HeaderMap::new();
        headers.insert(IF_NONE_MATCH, HeaderValue::from_static("*"));
        assert!(if_none_match_matches(&headers, "\"abc123\""));
    }

    #[test]
    fn ignores_unrelated_or_missing_headers() {
        let headers = HeaderMap::new();
        assert!(!if_none_match_matches(&headers, "\"abc123\""));

        let mut headers = HeaderMap::new();
        headers.insert(IF_NONE_MATCH, HeaderValue::from_static("\"nope\""));
        assert!(!if_none_match_matches(&headers, "\"abc123\""));
    }
}
