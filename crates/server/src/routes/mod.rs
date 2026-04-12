pub mod containers;
pub mod home;

use crate::AppState;
use axum::Router;

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/containers", containers::routes())
        .merge(home::routes())
        .nest("/lists", crate::lists::lists_router())
}
