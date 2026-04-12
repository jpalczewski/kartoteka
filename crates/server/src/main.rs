use axum::Router;
use kartoteka_server::{AppState, api_router};

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
    let pool = kartoteka_db::create_pool(&db_url)
        .await
        .expect("db connect");
    kartoteka_db::run_migrations(&pool)
        .await
        .expect("migrations");

    let state = AppState { pool };
    let app = Router::new().nest("/api", api_router()).with_state(state);

    let addr = "0.0.0.0:3000";
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
