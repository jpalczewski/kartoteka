pub mod containers;
pub mod home;

use crate::AppState;
use axum::Router;

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/containers", containers::routes())
        .merge(home::routes())
        .nest("/lists", crate::lists::lists_router())
        .nest("/lists/{list_id}/items", crate::items::list_items_router())
        .nest("/items", crate::items::items_router())
        .nest("/tags", crate::tags::tags_router())
        .nest("/tag-links", crate::tags::tag_links_router())
}
