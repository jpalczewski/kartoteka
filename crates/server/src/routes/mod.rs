pub mod containers;
pub mod home;

use axum::Router;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/containers", containers::routes())
        .merge(home::routes())
}
