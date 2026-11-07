//! Tests for the polymorphic-orphan cleanup (migration 017).
//!
//! `comments` and `entity_relations` point at items/lists/containers by
//! `(type, id)` with no foreign key, so nothing removed those rows when the
//! referenced entity was deleted. Migration 017 cleans the existing orphans and
//! installs AFTER DELETE triggers that keep both tables tidy.

use crate::SqlitePool;
use crate::test_helpers::{create_test_user, test_pool};
use uuid::Uuid;

async fn insert_list(pool: &SqlitePool, user_id: &str) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO lists (id, user_id, name) VALUES (?, ?, 'Test List')")
        .bind(&id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("insert_list");
    id
}

async fn insert_item(pool: &SqlitePool, list_id: &str, title: &str) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO items (id, list_id, title) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(list_id)
        .bind(title)
        .execute(pool)
        .await
        .expect("insert_item");
    id
}

async fn insert_container(pool: &SqlitePool, user_id: &str) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO containers (id, user_id, name) VALUES (?, ?, 'Test Container')")
        .bind(&id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("insert_container");
    id
}

async fn insert_comment(pool: &SqlitePool, user_id: &str, entity_type: &str, entity_id: &str) {
    sqlx::query(
        "INSERT INTO comments (id, entity_type, entity_id, content, user_id) \
         VALUES (?, ?, ?, 'hi', ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(entity_type)
    .bind(entity_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("insert_comment");
}

async fn insert_relation(
    pool: &SqlitePool,
    user_id: &str,
    from_type: &str,
    from_id: &str,
    to_type: &str,
    to_id: &str,
    relation_type: &str,
) {
    sqlx::query(
        "INSERT INTO entity_relations \
           (id, from_type, from_id, to_type, to_id, relation_type, user_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(from_type)
    .bind(from_id)
    .bind(to_type)
    .bind(to_id)
    .bind(relation_type)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("insert_relation");
}

async fn comment_count(pool: &SqlitePool, entity_type: &str, entity_id: &str) -> i64 {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM comments WHERE entity_type = ? AND entity_id = ?")
            .bind(entity_type)
            .bind(entity_id)
            .fetch_one(pool)
            .await
            .unwrap();
    row.0
}

async fn relation_count(pool: &SqlitePool) -> i64 {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM entity_relations")
        .fetch_one(pool)
        .await
        .unwrap();
    row.0
}

async fn delete_row(pool: &SqlitePool, table: &str, id: &str) {
    sqlx::query(&format!("DELETE FROM {table} WHERE id = ?"))
        .bind(id)
        .execute(pool)
        .await
        .expect("delete_row");
}

#[tokio::test]
async fn deleting_item_removes_its_comments_and_relations() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let list_id = insert_list(&pool, &user_id).await;
    let item_a = insert_item(&pool, &list_id, "A").await;
    let item_b = insert_item(&pool, &list_id, "B").await;
    let item_c = insert_item(&pool, &list_id, "C").await;

    insert_comment(&pool, &user_id, "item", &item_a).await;
    insert_comment(&pool, &user_id, "item", &item_b).await;
    insert_comment(&pool, &user_id, "list", &list_id).await;

    insert_relation(&pool, &user_id, "item", &item_a, "item", &item_b, "blocks").await;
    insert_relation(
        &pool,
        &user_id,
        "item",
        &item_b,
        "item",
        &item_a,
        "relates_to",
    )
    .await;
    insert_relation(&pool, &user_id, "item", &item_b, "item", &item_c, "blocks").await;

    delete_row(&pool, "items", &item_a).await;

    assert_eq!(comment_count(&pool, "item", &item_a).await, 0);
    assert_eq!(comment_count(&pool, "item", &item_b).await, 1);
    assert_eq!(comment_count(&pool, "list", &list_id).await, 1);
    // Only the unrelated b -> c relation is left.
    assert_eq!(relation_count(&pool).await, 1);
}

#[tokio::test]
async fn deleting_list_removes_comments_and_relations_of_it_and_its_items() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let list_id = insert_list(&pool, &user_id).await;
    let other_list_id = insert_list(&pool, &user_id).await;
    let item_a = insert_item(&pool, &list_id, "A").await;
    let item_b = insert_item(&pool, &other_list_id, "B").await;

    insert_comment(&pool, &user_id, "list", &list_id).await;
    insert_comment(&pool, &user_id, "item", &item_a).await;
    insert_comment(&pool, &user_id, "list", &other_list_id).await;
    insert_comment(&pool, &user_id, "item", &item_b).await;

    insert_relation(
        &pool,
        &user_id,
        "list",
        &list_id,
        "item",
        &item_b,
        "relates_to",
    )
    .await;
    insert_relation(&pool, &user_id, "item", &item_a, "item", &item_b, "blocks").await;
    insert_relation(
        &pool,
        &user_id,
        "item",
        &item_b,
        "item",
        &item_b,
        "relates_to",
    )
    .await;

    delete_row(&pool, "lists", &list_id).await;

    assert_eq!(comment_count(&pool, "list", &list_id).await, 0);
    // The list delete cascades to item_a, whose trigger cleans up after it.
    assert_eq!(comment_count(&pool, "item", &item_a).await, 0);
    // Unrelated rows stay.
    assert_eq!(comment_count(&pool, "list", &other_list_id).await, 1);
    assert_eq!(comment_count(&pool, "item", &item_b).await, 1);
    assert_eq!(relation_count(&pool).await, 1);
}

#[tokio::test]
async fn deleting_container_removes_its_comments_and_relations() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let container_id = insert_container(&pool, &user_id).await;
    let list_id = insert_list(&pool, &user_id).await;

    insert_comment(&pool, &user_id, "container", &container_id).await;
    insert_comment(&pool, &user_id, "list", &list_id).await;
    insert_relation(
        &pool,
        &user_id,
        "list",
        &list_id,
        "container",
        &container_id,
        "belongs_to",
    )
    .await;

    delete_row(&pool, "containers", &container_id).await;

    assert_eq!(comment_count(&pool, "container", &container_id).await, 0);
    assert_eq!(relation_count(&pool).await, 0);
    assert_eq!(comment_count(&pool, "list", &list_id).await, 1);
}

#[tokio::test]
async fn migration_deletes_orphans_that_already_exist() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let list_id = insert_list(&pool, &user_id).await;
    let item_id = insert_item(&pool, &list_id, "A").await;

    // Rows pointing at entities that never existed.
    insert_comment(&pool, &user_id, "item", "ghost-item").await;
    insert_comment(&pool, &user_id, "list", "ghost-list").await;
    insert_comment(&pool, &user_id, "item", &item_id).await;
    insert_relation(
        &pool,
        &user_id,
        "item",
        "ghost-item",
        "item",
        &item_id,
        "blocks",
    )
    .await;
    insert_relation(
        &pool,
        &user_id,
        "item",
        &item_id,
        "list",
        "ghost-list",
        "blocks",
    )
    .await;
    insert_relation(
        &pool,
        &user_id,
        "item",
        &item_id,
        "item",
        &item_id,
        "relates_to",
    )
    .await;

    // Re-run 017's cleanup: the migration itself already ran on this pool.
    let migrator = sqlx::migrate!("./migrations");
    let migration = migrator
        .iter()
        .find(|m| m.version == 17)
        .expect("migration 017 is embedded");
    sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();

    assert_eq!(comment_count(&pool, "item", "ghost-item").await, 0);
    assert_eq!(comment_count(&pool, "list", "ghost-list").await, 0);
    assert_eq!(comment_count(&pool, "item", &item_id).await, 1);
    assert_eq!(relation_count(&pool).await, 1);
}
