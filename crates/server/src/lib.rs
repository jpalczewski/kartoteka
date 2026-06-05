pub mod auth;
pub mod error;
pub mod extractors;
pub mod health;
pub mod items;
pub mod lists;
pub mod relations;
pub mod routes;
pub mod search;
pub mod settings;
pub mod tags;
pub mod templates;
pub mod time_entries;

pub use error::AppError;
pub use extractors::UserId;

use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    response::{IntoResponse, Response},
};
use kartoteka_db::SqlitePool;
use kartoteka_frontend::{App, shell};
use kartoteka_mcp::McpI18n;
use kartoteka_oauth::OAuthState;
use leptos::prelude::*;
use leptos_axum::{AxumRouteListing, LeptosRoutes, generate_route_list};
use std::sync::Arc;

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
    pub oauth_state: OAuthState,
    pub mcp_i18n: Arc<McpI18n>,
}

/// Type alias for the axum-login auth manager layer.
pub type AuthLayer = axum_login::AuthManagerLayer<
    kartoteka_auth::KartotekaBackend,
    tower_sessions_sqlx_store::SqliteStore,
>;

/// Handles Leptos server function calls at `/leptos/{*fn_name}`.
/// All `#[server]` functions in frontend must use `#[server(prefix = "/leptos")]`.
pub async fn server_fn_handler(
    State(state): State<AppState>,
    req: Request<Body>,
) -> impl IntoResponse {
    leptos_axum::handle_server_fns_with_context(
        move || {
            provide_context(state.pool.clone());
            provide_context(state.clone());
            provide_context(kartoteka_frontend::server_fns::settings::SigningSecret(
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
            provide_context(kartoteka_frontend::server_fns::settings::SigningSecret(
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
        "http://localhost:3000".into(),
        leptos::config::LeptosOptions::builder()
            .output_name("kartoteka")
            .build(),
        Arc::new(McpI18n::load_from(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../locales"),
        )),
    )
}

pub fn router(
    pool: SqlitePool,
    auth_layer: AuthLayer,
    signing_secret: String,
    public_base_url: String,
    leptos_options: leptos::config::LeptosOptions,
    mcp_i18n: Arc<McpI18n>,
) -> Router {
    use rmcp::transport::streamable_http_server::{
        StreamableHttpService, session::local::LocalSessionManager,
    };

    let routes = generate_route_list(App);
    let oauth_state = OAuthState {
        pool: pool.clone(),
        signing_secret: signing_secret.clone(),
        public_base_url,
    };
    let state = AppState {
        leptos_options: leptos_options.clone(),
        routes: routes.clone(),
        pool: pool.clone(),
        signing_secret,
        oauth_state: oauth_state.clone(),
        mcp_i18n: mcp_i18n.clone(),
    };

    let mcp_server = kartoteka_mcp::KartotekaServer::new(pool.clone(), mcp_i18n);
    let mcp_service = StreamableHttpService::new(
        move || Ok(mcp_server.clone()),
        Arc::new(LocalSessionManager::default()),
        Default::default(),
    );

    Router::new()
        .nest(
            "/.well-known",
            kartoteka_oauth::well_known_routes().with_state(oauth_state.clone()),
        )
        .nest(
            "/oauth",
            kartoteka_oauth::routes().with_state(oauth_state.clone()),
        )
        .nest_service(
            "/mcp",
            tower::ServiceBuilder::new()
                .layer(axum::middleware::from_fn_with_state(
                    oauth_state.clone(),
                    kartoteka_oauth::bearer::bearer_auth_middleware,
                ))
                .service(mcp_service),
        )
        .nest("/auth", auth::auth_router())
        .nest("/api", routes::routes(state.clone()))
        // Server functions: /leptos/{fn_name} (avoids conflict with /api/* REST routes)
        .route(
            "/leptos/{*fn_name}",
            axum::routing::get(server_fn_handler).post(server_fn_handler),
        )
        .route("/health", axum::routing::get(health::health))
        // SSR page rendering for all Leptos routes (/, /today, /lists/:id, etc.)
        .leptos_routes_with_handler(routes, axum::routing::get(leptos_routes_handler))
        // Static asset serving from target/site
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
        .layer(auth_layer)
        .with_state(state)
}
