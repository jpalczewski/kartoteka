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

    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into());
    let app = kartoteka_server::router(pool);

    tracing::info!("listening on {bind_addr}");
    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
