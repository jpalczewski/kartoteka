use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "kartoteka_server=debug,kartoteka_domain=debug,kartoteka_db=debug,tower_http=debug"
                    .into()
            }),
        )
        .pretty()
        .init();

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data.db".into());
    let pool = kartoteka_db::create_pool(&db_url).await.expect("db connect");
    kartoteka_db::run_migrations(&pool).await.expect("migrations");

    let session_store = SqliteStore::new(pool.clone());
    session_store.migrate().await.expect("session store migrate");

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(7)));

    let backend = kartoteka_auth::KartotekaBackend::new(pool.clone());
    let auth_layer = axum_login::AuthManagerLayerBuilder::new(backend, session_layer).build();

    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into());
    let app = kartoteka_server::router(pool, auth_layer);

    tracing::info!("listening on {bind_addr}");
    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
