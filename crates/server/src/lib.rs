pub mod auth;
pub mod error;
pub mod extractors;
pub mod items;
pub mod lists;
pub mod settings;
pub mod tags;
pub mod routes;

pub use error::AppError;
pub use extractors::UserId;

use axum::Router;
use kartoteka_db::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

/// Type alias to avoid verbose generic in function signatures.
pub type AuthLayer = axum_login::AuthManagerLayer<
    kartoteka_auth::KartotekaBackend,
    tower_sessions_sqlx_store::SqliteStore,
>;

pub fn router(pool: SqlitePool, auth_layer: AuthLayer) -> Router {
    let state = AppState { pool };
    Router::new()
        .nest("/auth", auth::auth_router())
        .nest("/api", routes::routes())
        .layer(auth_layer)
        .with_state(state)
}
