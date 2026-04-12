pub mod error;
pub mod extractors;
pub mod routes;

use axum::Router;
use kartoteka_db::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

pub fn router(pool: SqlitePool) -> Router {
    let state = AppState { pool };
    Router::new()
        .nest("/api", routes::routes())
        .with_state(state)
}
