use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{HeaderValue, Request, StatusCode, Uri, header::CACHE_CONTROL},
    middleware::Next,
    response::{IntoResponse, Response},
};
use tower_http::trace::MakeSpan;
use tracing::{Level, Span};

/// Paths whose segments legitimately contain dots (e.g. /pkg/app-hash.js).
/// `/cal/` serves `/cal/{token}/feed.ics`, whose last segment is a filename.
const DOTTED_SEGMENT_ALLOWLIST: &[&str] = &["/pkg/", "/cal/"];

/// Prefix of the public calendar feed route — everything after it is secret.
const CAL_PREFIX: &str = "/cal/";
/// Replacement for the redacted path segment.
const REDACTED_SEGMENT: &str = "[redacted]";

/// Rejects requests where a nested URL path segment looks like a filename (contains a dot
/// but does not start with one). App-level IDs are UUIDs and never contain dots;
/// static-file names always do. Short-circuits before Leptos SSR runs DB queries.
///
/// Root-level paths like `/favicon.ico` are left through — only nested segments
/// (depth ≥ 2) are checked, so `/lists/installHook.js.map` is caught but `/favicon.ico`
/// is not.
pub async fn reject_file_like_paths(req: Request<Body>, next: Next) -> Response {
    let path = req.uri().path();

    if DOTTED_SEGMENT_ALLOWLIST
        .iter()
        .any(|prefix| path.starts_with(prefix))
    {
        return next.run(req).await;
    }

    // Skip the first non-empty segment so root-level static files (/favicon.ico,
    // /robots.txt) pass through to the fallback handler.
    let has_file_like_nested = path
        .split('/')
        .filter(|s| !s.is_empty())
        .skip(1)
        .any(is_file_like_segment);

    if has_file_like_nested {
        return StatusCode::NOT_FOUND.into_response();
    }

    next.run(req).await
}

/// `Cache-Control` for content-hashed build assets: a new build gets new file names,
/// so a given URL never changes and browsers and CDNs may keep it for a year.
const IMMUTABLE_CACHE: &str = "public, max-age=31536000, immutable";

/// Marks successful responses under `pkg_prefix` (e.g. `/pkg/`) as immutable.
///
/// Only correct while `LEPTOS_HASH_FILES` is on: without hashed names the same URL
/// would carry a new build and clients would keep the old one. [`crate::router`]
/// adds this layer only in that case. Errors (e.g. a 404 for a file from a previous
/// build) are left uncached, so they don't stick once the file exists.
pub async fn immutable_hashed_assets(
    State(pkg_prefix): State<Arc<str>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let is_asset = req.uri().path().starts_with(&*pkg_prefix);
    let mut resp = next.run(req).await;
    if is_asset && resp.status().is_success() {
        resp.headers_mut()
            .insert(CACHE_CONTROL, HeaderValue::from_static(IMMUTABLE_CACHE));
    }
    resp
}

/// A segment is "file-like" if it contains a dot but does not start with one.
/// Segments starting with `.` are directory names like `.well-known`, not files.
fn is_file_like_segment(segment: &str) -> bool {
    !segment.is_empty() && !segment.starts_with('.') && segment.contains('.')
}

/// Request span for [`crate::router`]: the same INFO-level fields as
/// `DefaultMakeSpan` (`method`, `uri`, `version`), but with the calendar feed
/// token redacted from the URI.
///
/// `TraceLayer` records the full URI, and `/cal/{token}/feed.ics` carries a
/// bearer-equivalent secret in the path — logging it would hand out feed access
/// to anyone with log access.
#[derive(Debug, Clone, Copy)]
pub struct RedactingMakeSpan {
    level: Level,
}

impl Default for RedactingMakeSpan {
    fn default() -> Self {
        Self { level: Level::INFO }
    }
}

impl RedactingMakeSpan {
    pub const fn new(level: Level) -> Self {
        Self { level }
    }
}

impl<B> MakeSpan<B> for RedactingMakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let uri = redact_uri(request.uri());
        let method = request.method();
        let version = request.version();
        // `tracing` needs a literal level per macro, hence the dispatch.
        match self.level {
            Level::ERROR => {
                tracing::error_span!("request", method = %method, uri = %uri, version = ?version)
            }
            Level::WARN => {
                tracing::warn_span!("request", method = %method, uri = %uri, version = ?version)
            }
            Level::INFO => {
                tracing::info_span!("request", method = %method, uri = %uri, version = ?version)
            }
            Level::DEBUG => {
                tracing::debug_span!("request", method = %method, uri = %uri, version = ?version)
            }
            Level::TRACE => {
                tracing::trace_span!("request", method = %method, uri = %uri, version = ?version)
            }
        }
    }
}

/// The URI as logged: identical to the request URI, except that the calendar
/// token is replaced — `/cal/{token}/feed.ics` becomes `/cal/[redacted]/feed.ics`.
pub fn redact_uri(uri: &Uri) -> String {
    let Some(rest) = uri.path().strip_prefix(CAL_PREFIX) else {
        return uri.to_string();
    };
    let tail = match rest.split_once('/') {
        Some((_, tail)) => format!("/{tail}"),
        None => String::new(),
    };
    let mut redacted = format!("{CAL_PREFIX}{REDACTED_SEGMENT}{tail}");
    if let Some(query) = uri.query() {
        redacted.push('?');
        redacted.push_str(query);
    }
    redacted
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::Request, routing::get};
    use tower::ServiceExt;

    async fn dummy() -> &'static str {
        "ok"
    }

    fn app() -> Router {
        Router::new()
            .route("/lists/{id}", get(dummy))
            .route("/pkg/{*file}", get(dummy))
            .layer(axum::middleware::from_fn(reject_file_like_paths))
    }

    #[tokio::test]
    async fn rejects_source_map_path() {
        let resp = app()
            .oneshot(
                Request::builder()
                    .uri("/lists/installHook.js.map")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 404);
    }

    #[tokio::test]
    async fn rejects_js_path() {
        let resp = app()
            .oneshot(
                Request::builder()
                    .uri("/lists/app.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 404);
    }

    #[tokio::test]
    async fn allows_uuid_path() {
        let resp = app()
            .oneshot(
                Request::builder()
                    .uri("/lists/a1b2c3d4-0000-4000-8000-000000000000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn allows_pkg_assets() {
        let resp = app()
            .oneshot(
                Request::builder()
                    .uri("/pkg/app-abc123.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn allows_root_level_favicon() {
        let app = Router::new()
            .route("/favicon.ico", get(dummy))
            .layer(axum::middleware::from_fn(reject_file_like_paths));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/favicon.ico")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn allows_calendar_feed_path() {
        let app = Router::new()
            .route("/cal/{token}/feed.ics", get(dummy))
            .layer(axum::middleware::from_fn(reject_file_like_paths));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/cal/abc/feed.ics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[test]
    fn redacts_calendar_token_in_uri() {
        let uri: axum::http::Uri = "/cal/AbC-123_xyz/feed.ics".parse().unwrap();
        assert_eq!(redact_uri(&uri), "/cal/[redacted]/feed.ics");
    }

    #[test]
    fn redacts_token_and_keeps_query() {
        let uri: axum::http::Uri = "/cal/secret/feed.ics?refresh=1".parse().unwrap();
        assert_eq!(redact_uri(&uri), "/cal/[redacted]/feed.ics?refresh=1");
    }

    #[test]
    fn redacts_token_without_tail() {
        let uri: axum::http::Uri = "/cal/secret".parse().unwrap();
        assert_eq!(redact_uri(&uri), "/cal/[redacted]");
    }

    #[test]
    fn leaves_other_uris_untouched() {
        for raw in ["/api/lists?archived=true", "/health", "/pkg/app-abc.js"] {
            let uri: axum::http::Uri = raw.parse().unwrap();
            assert_eq!(redact_uri(&uri), raw);
        }
    }

    #[tokio::test]
    async fn allows_well_known() {
        let app = Router::new()
            .route("/.well-known/openid-configuration", get(dummy))
            .layer(axum::middleware::from_fn(reject_file_like_paths));
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/.well-known/openid-configuration")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    fn cached_app() -> Router {
        Router::new()
            .route("/pkg/app-abc123.wasm", get(dummy))
            .route("/lists", get(dummy))
            .layer(axum::middleware::from_fn_with_state(
                Arc::<str>::from("/pkg/"),
                immutable_hashed_assets,
            ))
    }

    async fn cache_control(uri: &str) -> (StatusCode, Option<String>) {
        let resp = cached_app()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let cc = resp
            .headers()
            .get(CACHE_CONTROL)
            .map(|v| v.to_str().unwrap().to_owned());
        (resp.status(), cc)
    }

    #[tokio::test]
    async fn pkg_asset_is_immutable() {
        assert_eq!(
            cache_control("/pkg/app-abc123.wasm").await,
            (StatusCode::OK, Some(IMMUTABLE_CACHE.to_owned()))
        );
    }

    #[tokio::test]
    async fn missing_pkg_asset_is_not_cached() {
        assert_eq!(
            cache_control("/pkg/app-old.wasm").await,
            (StatusCode::NOT_FOUND, None)
        );
    }

    #[tokio::test]
    async fn pages_are_not_cached() {
        assert_eq!(cache_control("/lists").await, (StatusCode::OK, None));
    }
}
