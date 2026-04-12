use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous,
};
use std::str::FromStr;

pub use sqlx::sqlite::SqlitePool;

pub mod containers;
pub mod test_helpers;
pub mod types;

/// Database error type.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("not found: {0}")]
    NotFound(&'static str),
}

/// Create a SQLite connection pool with WAL mode, pragmas, and tuning.
///
/// Pass `":memory:"` for in-memory (tests), or a file path for persistent storage.
pub async fn create_pool(url: &str) -> Result<SqlitePool, DbError> {
    let options = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .min_connections(2)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("PRAGMA busy_timeout = 5000")
                    .execute(&mut *conn)
                    .await?;
                sqlx::query("PRAGMA mmap_size = 268435456")
                    .execute(&mut *conn)
                    .await?;
                sqlx::query("PRAGMA optimize = 0x10002")
                    .execute(&mut *conn)
                    .await?;
                Ok(())
            })
        })
        .connect_with(options)
        .await?;

    Ok(pool)
}

/// Run all embedded migrations against the pool.
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), DbError> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pool_connects_and_migrates() {
        let pool = create_pool(":memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 0);
    }

    #[tokio::test]
    async fn migration_creates_server_config_default() {
        let pool = create_pool(":memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();

        let row: (String,) =
            sqlx::query_as("SELECT value FROM server_config WHERE key = 'registration_enabled'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, "true");
    }

    #[tokio::test]
    async fn migration_creates_fts5_tables() {
        let pool = create_pool(":memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();

        sqlx::query("INSERT INTO users (id, email) VALUES ('u1', 'test@test.com')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES ('l1', 'u1', 'Test List')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO items (id, list_id, title, description) VALUES ('i1', 'l1', 'Buy milk', 'whole milk')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let results: Vec<(String,)> =
            sqlx::query_as("SELECT title FROM items_fts WHERE items_fts MATCH 'milk'")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "Buy milk");
    }
}
