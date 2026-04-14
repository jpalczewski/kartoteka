pub mod containers;
pub mod home;

use crate::{AppState, auth};
use axum::{Router, middleware};

pub fn routes(state: AppState) -> Router<AppState> {
    let admin_routes =
        crate::auth::server_config_router().route_layer(middleware::from_fn(auth::require_admin));

    Router::new()
        .nest("/containers", containers::routes())
        .merge(home::routes())
        .nest("/lists", crate::lists::lists_router())
        .nest("/lists/{list_id}/items", crate::items::list_items_router())
        .nest("/items", crate::items::items_router())
        .nest("/tags", crate::tags::tags_router())
        .nest("/tag-links", crate::tags::tag_links_router())
        .nest("/settings", crate::settings::settings_router())
        .nest("/preferences", crate::settings::preferences_router())
        .nest("/server-config", admin_routes)
        .route_layer(middleware::from_fn_with_state(state, auth::require_auth))
}
