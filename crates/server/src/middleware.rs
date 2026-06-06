use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Paths whose segments legitimately contain dots (e.g. /pkg/app-hash.js).
const DOTTED_SEGMENT_ALLOWLIST: &[&str] = &["/pkg/"];

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

/// A segment is "file-like" if it contains a dot but does not start with one.
/// Segments starting with `.` are directory names like `.well-known`, not files.
fn is_file_like_segment(segment: &str) -> bool {
    !segment.is_empty() && !segment.starts_with('.') && segment.contains('.')
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
}
