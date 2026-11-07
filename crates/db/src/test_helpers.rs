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

/// Insert a minimal test list (owned by `user_id`) and return its ID.
pub async fn create_test_list(pool: &SqlitePool, user_id: &str) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO lists (id, user_id, name, list_type) VALUES (?, ?, 'Test List', 'checklist')",
    )
    .bind(&id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("create_test_list");
    id
}

/// Insert a minimal test item into `list_id` and return its ID.
pub async fn create_test_item(pool: &SqlitePool, list_id: &str) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO items (id, list_id, title) VALUES (?, ?, 'Test Item')")
        .bind(&id)
        .bind(list_id)
        .execute(pool)
        .await
        .expect("create_test_item");
    id
}

/// Insert a minimal `basic` test tag (owned by `user_id`) and return its ID.
pub async fn create_test_tag(pool: &SqlitePool, user_id: &str, name: &str) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO tags (id, user_id, name, tag_type) VALUES (?, ?, ?, 'basic')")
        .bind(&id)
        .bind(user_id)
        .bind(name)
        .execute(pool)
        .await
        .expect("create_test_tag");
    id
}
