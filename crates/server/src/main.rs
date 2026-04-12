#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "kartoteka_server=debug,kartoteka_domain=debug,kartoteka_db=info,tower_http=debug"
                    .into()
            }),
        )
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "kartoteka.db".to_string());
    let pool = kartoteka_db::create_pool(&database_url)
        .await
        .expect("failed to create database pool");
    kartoteka_db::run_migrations(&pool)
        .await
        .expect("failed to run migrations");

    let app = kartoteka_server::router(pool);
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {bind_addr}: {e}"));
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
