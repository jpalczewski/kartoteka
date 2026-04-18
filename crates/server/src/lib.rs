pub mod auth;
pub mod error;
pub mod extractors;
pub mod items;
pub mod lists;
pub mod relations;
pub mod routes;
pub mod settings;
pub mod tags;

pub use error::AppError;
pub use extractors::UserId;

use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    response::{IntoResponse, Response},
};
use kartoteka_db::SqlitePool;
use kartoteka_frontend_v2::{App, shell};
use leptos::prelude::*;
use leptos_axum::{AxumRouteListing, LeptosRoutes, generate_route_list};

/// Application state shared across all handlers.
/// `axum_macros::FromRef` allows individual fields to be extracted as `State<T>`.
#[derive(Clone, axum_macros::FromRef)]
pub struct AppState {
    pub leptos_options: leptos::config::LeptosOptions,
    /// Pre-generated Leptos route list (reused per request by leptos_routes_handler).
    pub routes: Vec<AxumRouteListing>,
    pub pool: SqlitePool,
    /// HMAC-SHA256 signing secret for JWT bearer tokens (min 32 chars).
    pub signing_secret: String,
}

/// Type alias for the axum-login auth manager layer.
pub type AuthLayer = axum_login::AuthManagerLayer<
    kartoteka_auth::KartotekaBackend,
    tower_sessions_sqlx_store::SqliteStore,
>;

/// Handles Leptos server function calls at `/leptos/{*fn_name}`.
/// All `#[server]` functions in frontend-v2 must use `#[server(prefix = "/leptos")]`.
pub async fn server_fn_handler(
    State(state): State<AppState>,
    req: Request<Body>,
) -> impl IntoResponse {
    leptos_axum::handle_server_fns_with_context(
        move || {
            provide_context(state.pool.clone());
            provide_context(state.clone());
            provide_context(kartoteka_frontend_v2::server_fns::settings::SigningSecret(
                state.signing_secret.clone(),
            ));
        },
        req,
    )
    .await
}

/// Renders Leptos SSR pages for all frontend routes.
pub async fn leptos_routes_handler(State(state): State<AppState>, req: Request<Body>) -> Response {
    let options = state.leptos_options.clone();
    let pool = state.pool.clone();
    let s = state.clone();
    let signing_secret = state.signing_secret.clone();
    let handler = leptos_axum::render_route_with_context(
        state.routes.clone(),
        move || {
            provide_context(pool.clone());
            provide_context(s.clone());
            provide_context(kartoteka_frontend_v2::server_fns::settings::SigningSecret(
                signing_secret.clone(),
            ));
        },
        move || shell(options.clone()),
    );
    handler(axum::extract::State(state), req)
        .await
        .into_response()
}

/// Convenience wrapper for integration tests: creates a router with default LeptosOptions.
#[doc(hidden)]
pub fn test_router(pool: SqlitePool, auth_layer: AuthLayer, signing_secret: String) -> Router {
    router(
        pool,
        auth_layer,
        signing_secret,
        leptos::config::LeptosOptions::builder()
            .output_name("kartoteka")
            .build(),
    )
}

pub fn router(
    pool: SqlitePool,
    auth_layer: AuthLayer,
    signing_secret: String,
    leptos_options: leptos::config::LeptosOptions,
) -> Router {
    let routes = generate_route_list(App);

    let state = AppState {
        leptos_options: leptos_options.clone(),
        routes: routes.clone(),
        pool,
        signing_secret,
    };

    Router::new()
        .nest("/auth", auth::auth_router())
        .nest("/api", routes::routes(state.clone()))
        // Server functions: /leptos/{fn_name} (avoids conflict with /api/* REST routes)
        .route(
            "/leptos/{*fn_name}",
            axum::routing::get(server_fn_handler).post(server_fn_handler),
        )
        // SSR page rendering for all Leptos routes (/, /today, /lists/:id, etc.)
        .leptos_routes_with_handler(routes, axum::routing::get(leptos_routes_handler))
        // Static asset serving from target/site
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
        .layer(auth_layer)
        .with_state(state)
}
