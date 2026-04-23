use crate::{OAuthState, storage};
use axum::{
    extract::State,
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use kartoteka_shared::auth_ctx::{UserId, UserLocale};

pub async fn bearer_auth_middleware(
    State(state): State<OAuthState>,
    mut req: axum::extract::Request,
    next: Next,
) -> Result<Response, Response> {
    let hdr = req.headers().get(header::AUTHORIZATION);
    let token = hdr
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| unauthorized(&state.public_base_url))?;

    let claims = storage::verify_access_token(token, &state.signing_secret)
        .map_err(|_| unauthorized(&state.public_base_url))?;

    if claims.scope != "mcp" {
        return Err((StatusCode::FORBIDDEN, "forbidden: wrong scope").into_response());
    }

    req.extensions_mut().insert(UserId(claims.sub));

    let locale = req
        .headers()
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .and_then(|v| v.split('-').next())
        .map(|s| s.to_lowercase())
        .filter(|s| s == "pl" || s == "en")
        .unwrap_or_else(|| "en".into());
    req.extensions_mut().insert(UserLocale(locale));

    Ok(next.run(req).await)
}

fn unauthorized(base: &str) -> Response {
    let mut r = (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    let value =
        format!(r#"Bearer resource_metadata="{base}/.well-known/oauth-protected-resource""#);
    r.headers_mut()
        .insert(header::WWW_AUTHENTICATE, value.parse().unwrap());
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, extract::Extension, routing::get};
    use http::Request;
    use kartoteka_db::test_helpers::test_pool;
    use tower::ServiceExt;

    async fn handler(
        Extension(uid): Extension<UserId>,
        Extension(loc): Extension<UserLocale>,
    ) -> String {
        format!("{}:{}", uid.0, loc.0)
    }

    fn app_with_state(state: OAuthState) -> Router {
        Router::new()
            .route("/t", get(handler))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                bearer_auth_middleware,
            ))
            .with_state(state)
    }

    #[tokio::test]
    async fn valid_bearer_sets_extensions() {
        let pool = test_pool().await;
        let secret = "secret-at-least-32-chars-long-padded";
        let state = OAuthState {
            pool,
            signing_secret: secret.into(),
            public_base_url: "http://x".into(),
        };
        let token = crate::storage::sign_access_token("u-1", "mcp", secret).unwrap();
        let req = Request::builder()
            .uri("/t")
            .header("authorization", format!("Bearer {token}"))
            .header("accept-language", "pl-PL,en;q=0.9")
            .body(Body::empty())
            .unwrap();
        let res = app_with_state(state).oneshot(req).await.unwrap();
        assert_eq!(res.status(), 200);
        let body = axum::body::to_bytes(res.into_body(), 512).await.unwrap();
        assert_eq!(std::str::from_utf8(&body).unwrap(), "u-1:pl");
    }

    #[tokio::test]
    async fn missing_bearer_returns_401() {
        let pool = test_pool().await;
        let state = OAuthState {
            pool,
            signing_secret: "x".repeat(32),
            public_base_url: "http://x".into(),
        };
        let req = Request::builder().uri("/t").body(Body::empty()).unwrap();
        let res = app_with_state(state).oneshot(req).await.unwrap();
        assert_eq!(res.status(), 401);
        assert!(res.headers().get("www-authenticate").is_some());
    }

    #[tokio::test]
    async fn wrong_scope_returns_403() {
        let pool = test_pool().await;
        let secret = "secret-at-least-32-chars-long-padded";
        let state = OAuthState {
            pool,
            signing_secret: secret.into(),
            public_base_url: "http://x".into(),
        };
        let token = crate::storage::sign_access_token("u-1", "calendar", secret).unwrap();
        let req = Request::builder()
            .uri("/t")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let res = app_with_state(state).oneshot(req).await.unwrap();
        assert_eq!(res.status(), 403);
    }
}
