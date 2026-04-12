use sqlx::SqlitePool;

pub mod lists;
pub mod test_helpers;

pub use kartoteka_shared::types::FlexDate;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub async fn create_pool(url: &str) -> Result<SqlitePool, DbError> {
    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
    use std::str::FromStr;

    let options = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);

    SqlitePoolOptions::new()
        .max_connections(8)
        .min_connections(2)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("PRAGMA busy_timeout = 5000").execute(&mut *conn).await?;
                sqlx::query("PRAGMA mmap_size = 268435456").execute(&mut *conn).await?;
                Ok(())
            })
        })
        .connect_with(options)
        .await
        .map_err(DbError::Sqlx)
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}
