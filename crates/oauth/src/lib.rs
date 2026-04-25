//! OAuth 2.1 provider — DCR, authorization-code + PKCE S256, refresh token rotation.
//! Bearer middleware validates JWT access tokens and injects UserId/UserLocale.

pub mod bearer;
pub mod errors;
pub mod handlers;
pub mod pkce;
pub mod storage;
pub mod types;

use axum::Router;
use kartoteka_db::SqlitePool;

#[derive(Clone)]
pub struct OAuthState {
    pub pool: SqlitePool,
    pub signing_secret: String,
    pub public_base_url: String,
}

/// Mount on `/oauth` in server/lib.rs.
pub fn routes() -> Router<OAuthState> {
    Router::new()
        .route(
            "/authorize",
            axum::routing::get(handlers::authorize_get).post(handlers::authorize_post),
        )
        .route("/token", axum::routing::post(handlers::token))
        .route("/register", axum::routing::post(handlers::register))
}

/// Mount on `/.well-known` in server/lib.rs.
pub fn well_known_routes() -> Router<OAuthState> {
    Router::new()
        .route(
            "/oauth-authorization-server",
            axum::routing::get(handlers::metadata_as),
        )
        .route(
            "/oauth-protected-resource",
            axum::routing::get(handlers::metadata_pr),
        )
}
