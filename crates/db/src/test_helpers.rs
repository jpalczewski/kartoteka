//! Test helpers — compiled when running tests or when the "test-helpers" feature is enabled.
//! Use in other crates' dev-dependencies: kartoteka-db = { path = "../db", features = ["test-helpers"] }

use crate::{create_pool, run_migrations};
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

/// Create an in-memory SQLite pool with all migrations applied. For tests only.
pub async fn test_pool() -> SqlitePool {
    let pool = create_pool(":memory:").await.expect("test pool creation");
    run_migrations(&pool).await.expect("test migrations");
    pool
}

/// Insert a minimal test user and return their ID.
pub async fn create_test_user(pool: &SqlitePool) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO users (id, email, name, role) VALUES (?, ?, 'Test User', 'user')")
        .bind(&id)
        .bind(format!("{}@test.local", &id[..8]))
        .execute(pool)
        .await
        .expect("create_test_user");
    id
}
