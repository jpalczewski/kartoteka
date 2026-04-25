use kartoteka_mcp::McpI18n;
use std::sync::Arc;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;

#[tokio::main]
async fn main() {
    let default_filter =
        "kartoteka_server=debug,kartoteka_domain=debug,kartoteka_db=debug,tower_http=debug";
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| default_filter.into());

    if std::env::var("APP_ENV").as_deref() == Ok("production") {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .json()
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .pretty()
            .init();
    }

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data.db".into());
    tracing::info!("connecting to database: {}", db_url);
    let pool = kartoteka_db::create_pool(&db_url)
        .await
        .expect("db connect");
    tracing::info!("running migrations");
    kartoteka_db::run_migrations(&pool)
        .await
        .expect("migrations");
    tracing::info!("migrations done");

    let session_store = SqliteStore::new(pool.clone());
    session_store
        .migrate()
        .await
        .expect("session store migrate");

    let secure_cookies = std::env::var("APP_ENV").as_deref() == Ok("production")
        || std::env::var("PUBLIC_BASE_URL")
            .map(|u| u.starts_with("https://"))
            .unwrap_or(false);
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(secure_cookies)
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(7)));

    let backend = kartoteka_auth::KartotekaBackend::new(pool.clone());
    let auth_layer = axum_login::AuthManagerLayerBuilder::new(backend, session_layer).build();

    let signing_secret = std::env::var("OAUTH_SIGNING_SECRET")
        .expect("OAUTH_SIGNING_SECRET env var must be set (min 32 chars, random)");
    assert!(
        signing_secret.len() >= 32,
        "OAUTH_SIGNING_SECRET must be at least 32 characters"
    );

    let public_base_url =
        std::env::var("PUBLIC_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".into());
    let mcp_i18n = Arc::new(McpI18n::load());

    let conf = leptos::config::get_configuration(None).expect("leptos config");
    let leptos_options = conf.leptos_options;
    let bind_addr =
        std::env::var("BIND_ADDR").unwrap_or_else(|_| leptos_options.site_addr.to_string());
    let app = kartoteka_server::router(
        pool,
        auth_layer,
        signing_secret,
        public_base_url,
        leptos_options,
        mcp_i18n,
    );

    tracing::info!("listening on {bind_addr}");
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {bind_addr}: {e}"));
    axum::serve(listener, app).await.expect("server error");
}
