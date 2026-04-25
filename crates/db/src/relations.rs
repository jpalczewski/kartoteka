use crate::{DbError, types::EntityRelationRow};
use sqlx::SqlitePool;

// ── Read queries ──────────────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn list_for_entity(
    pool: &SqlitePool,
    entity_type: &str,
    entity_id: &str,
    user_id: &str,
) -> Result<Vec<EntityRelationRow>, DbError> {
    sqlx::query_as::<_, EntityRelationRow>(
        "SELECT id, from_type, from_id, to_type, to_id, relation_type, user_id, created_at \
         FROM entity_relations \
         WHERE ((from_type = ? AND from_id = ?) OR (to_type = ? AND to_id = ?)) \
           AND user_id = ? \
         ORDER BY created_at ASC",
    )
    .bind(entity_type)
    .bind(entity_id)
    .bind(entity_type)
    .bind(entity_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn get_unresolved_blockers(
    pool: &SqlitePool,
    item_id: &str,
) -> Result<Vec<EntityRelationRow>, DbError> {
    sqlx::query_as::<_, EntityRelationRow>(
        "SELECT er.id, er.from_type, er.from_id, er.to_type, er.to_id, \
                er.relation_type, er.user_id, er.created_at \
         FROM entity_relations er \
         JOIN items i ON i.id = er.from_id \
         WHERE er.to_type = 'item' AND er.to_id = ? \
           AND er.relation_type = 'blocks' \
           AND er.from_type = 'item' \
           AND i.completed = 0",
    )
    .bind(item_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

// ── Write queries ─────────────────────────────────────────────────────────────

pub struct InsertRelationInput<'a> {
    pub id: &'a str,
    pub from_type: &'a str,
    pub from_id: &'a str,
    pub to_type: &'a str,
    pub to_id: &'a str,
    pub relation_type: &'a str,
    pub user_id: &'a str,
}

#[tracing::instrument(skip(pool, input), fields(from_type = %input.from_type, to_type = %input.to_type, relation_type = %input.relation_type))]
pub async fn insert(
    pool: &SqlitePool,
    input: InsertRelationInput<'_>,
) -> Result<EntityRelationRow, DbError> {
    sqlx::query_as::<_, EntityRelationRow>(
        "INSERT INTO entity_relations \
           (id, from_type, from_id, to_type, to_id, relation_type, user_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?) \
         RETURNING id, from_type, from_id, to_type, to_id, relation_type, user_id, created_at",
    )
    .bind(input.id)
    .bind(input.from_type)
    .bind(input.from_id)
    .bind(input.to_type)
    .bind(input.to_id)
    .bind(input.relation_type)
    .bind(input.user_id)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, relation_id: &str, user_id: &str) -> Result<bool, DbError> {
    let rows = sqlx::query("DELETE FROM entity_relations WHERE id = ? AND user_id = ?")
        .bind(relation_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_user, test_pool};
    use uuid::Uuid;

    async fn insert_test_list(pool: &SqlitePool, user_id: &str) -> String {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO lists (id, user_id, name, list_type) VALUES (?, ?, 'Test List', 'checklist')",
        )
        .bind(&id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("insert_test_list");
        id
    }

    async fn insert_test_item(pool: &SqlitePool, list_id: &str) -> String {
        let id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO items (id, list_id, title) VALUES (?, ?, 'Test Item')")
            .bind(&id)
            .bind(list_id)
            .execute(pool)
            .await
            .expect("insert_test_item");
        id
    }

    #[tokio::test]
    async fn insert_returns_relation() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &user_id).await;
        let item_a = insert_test_item(&pool, &list_id).await;
        let item_b = insert_test_item(&pool, &list_id).await;
        let rel_id = Uuid::new_v4().to_string();

        let row = insert(
            &pool,
            InsertRelationInput {
                id: &rel_id,
                from_type: "item",
                from_id: &item_a,
                to_type: "item",
                to_id: &item_b,
                relation_type: "blocks",
                user_id: &user_id,
            },
        )
        .await
        .unwrap();

        assert_eq!(row.id, rel_id);
        assert_eq!(row.from_id, item_a);
        assert_eq!(row.to_id, item_b);
        assert_eq!(row.relation_type, "blocks");
    }

    #[tokio::test]
    async fn list_for_entity_returns_both_directions() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &user_id).await;
        let item_a = insert_test_item(&pool, &list_id).await;
        let item_b = insert_test_item(&pool, &list_id).await;
        let item_c = insert_test_item(&pool, &list_id).await;

        insert(
            &pool,
            InsertRelationInput {
                id: &Uuid::new_v4().to_string(),
                from_type: "item",
                from_id: &item_a,
                to_type: "item",
                to_id: &item_b,
                relation_type: "blocks",
                user_id: &user_id,
            },
        )
        .await
        .unwrap();

        insert(
            &pool,
            InsertRelationInput {
                id: &Uuid::new_v4().to_string(),
                from_type: "item",
                from_id: &item_c,
                to_type: "item",
                to_id: &item_a,
                relation_type: "relates_to",
                user_id: &user_id,
            },
        )
        .await
        .unwrap();

        let rows = list_for_entity(&pool, "item", &item_a, &user_id)
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[tokio::test]
    async fn get_unresolved_blockers_returns_incomplete_only() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &user_id).await;
        let blocker = insert_test_item(&pool, &list_id).await;
        let blocked = insert_test_item(&pool, &list_id).await;

        insert(
            &pool,
            InsertRelationInput {
                id: &Uuid::new_v4().to_string(),
                from_type: "item",
                from_id: &blocker,
                to_type: "item",
                to_id: &blocked,
                relation_type: "blocks",
                user_id: &user_id,
            },
        )
        .await
        .unwrap();

        let rows = get_unresolved_blockers(&pool, &blocked).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].from_id, blocker);

        sqlx::query("UPDATE items SET completed = 1 WHERE id = ?")
            .bind(&blocker)
            .execute(&pool)
            .await
            .unwrap();

        let rows = get_unresolved_blockers(&pool, &blocked).await.unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn delete_removes_relation() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &user_id).await;
        let item_a = insert_test_item(&pool, &list_id).await;
        let item_b = insert_test_item(&pool, &list_id).await;
        let rel_id = Uuid::new_v4().to_string();

        insert(
            &pool,
            InsertRelationInput {
                id: &rel_id,
                from_type: "item",
                from_id: &item_a,
                to_type: "item",
                to_id: &item_b,
                relation_type: "blocks",
                user_id: &user_id,
            },
        )
        .await
        .unwrap();

        let deleted = delete(&pool, &rel_id, &user_id).await.unwrap();
        assert!(deleted);

        let rows = list_for_entity(&pool, "item", &item_a, &user_id)
            .await
            .unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn delete_wrong_user_returns_false() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let other_user = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &user_id).await;
        let item_a = insert_test_item(&pool, &list_id).await;
        let item_b = insert_test_item(&pool, &list_id).await;
        let rel_id = Uuid::new_v4().to_string();

        insert(
            &pool,
            InsertRelationInput {
                id: &rel_id,
                from_type: "item",
                from_id: &item_a,
                to_type: "item",
                to_id: &item_b,
                relation_type: "blocks",
                user_id: &user_id,
            },
        )
        .await
        .unwrap();

        let deleted = delete(&pool, &rel_id, &other_user).await.unwrap();
        assert!(!deleted);

        let rows = list_for_entity(&pool, "item", &item_a, &user_id)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
    }
}
