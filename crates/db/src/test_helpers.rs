//! Test helpers — compiled when running tests or when the "test-helpers" feature is enabled.
//! Use in other crates' dev-dependencies: kartoteka-db = { path = "../db", features = ["test-helpers"] }

use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

pub async fn create_test_user(pool: &SqlitePool) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO users (id, email, role) VALUES (?, ?, 'user')"
    )
    .bind(&id)
    .bind(format!("{}@test.com", &id[..8]))
    .execute(pool)
    .await
    .unwrap();
    id
}
