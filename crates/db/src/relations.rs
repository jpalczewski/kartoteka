use crate::{DbError, types::EntityRelationRow};
use sqlx::SqlitePool;

const RELATION_COLS: &str =
    "id, from_type, from_id, to_type, to_id, relation_type, user_id, created_at";

// ── Read queries ──────────────────────────────────────────────────────────────

/// A relation plus the display data of the entity on its *other* side.
#[derive(Debug, sqlx::FromRow)]
pub struct EntityRelationWithTitleRow {
    pub id: String,
    pub from_type: String,
    pub from_id: String,
    pub to_type: String,
    pub to_id: String,
    pub relation_type: String,
    pub user_id: String,
    pub created_at: String,
    /// `item` / `list` / `container` of the opposite side.
    pub other_type: String,
    /// `items.title`, `lists.name` or `containers.name` of the opposite side.
    pub other_title: Option<String>,
    /// `items.completed` — only set when the opposite side is an item.
    pub other_completed: Option<bool>,
}

/// All relations touching `entity_id`, each with the title (and, for items, the
/// completed flag) of the entity on the other side.
///
/// One query: the opposite side is resolved per row with a `CASE`, then joined
/// against items, lists and containers (all scoped to the relation's user).
#[tracing::instrument(skip(pool))]
pub async fn list_for_entity_with_titles(
    pool: &SqlitePool,
    entity_type: &str,
    entity_id: &str,
    user_id: &str,
) -> Result<Vec<EntityRelationWithTitleRow>, DbError> {
    sqlx::query_as::<_, EntityRelationWithTitleRow>(
        "WITH rel AS ( \
           SELECT er.id, er.from_type, er.from_id, er.to_type, er.to_id, er.relation_type, \
                  er.user_id, er.created_at, \
                  CASE WHEN er.from_type = ?1 AND er.from_id = ?2 THEN er.to_type ELSE er.from_type END AS other_type, \
                  CASE WHEN er.from_type = ?1 AND er.from_id = ?2 THEN er.to_id ELSE er.from_id END AS other_id \
           FROM entity_relations er \
           WHERE ((er.from_type = ?1 AND er.from_id = ?2) OR (er.to_type = ?1 AND er.to_id = ?2)) \
             AND er.user_id = ?3 \
         ) \
         SELECT rel.id, rel.from_type, rel.from_id, rel.to_type, rel.to_id, rel.relation_type, \
                rel.user_id, rel.created_at, rel.other_type, \
                COALESCE(i.title, l.name, c.name) AS other_title, \
                i.completed AS other_completed \
         FROM rel \
         LEFT JOIN items i ON i.id = rel.other_id \
              AND EXISTS (SELECT 1 FROM lists li WHERE li.id = i.list_id AND li.user_id = rel.user_id) \
         LEFT JOIN lists l ON l.id = rel.other_id AND l.user_id = rel.user_id \
         LEFT JOIN containers c ON c.id = rel.other_id AND c.user_id = rel.user_id \
         ORDER BY rel.created_at ASC",
    )
    .bind(entity_type)
    .bind(entity_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn list_for_entity(
    pool: &SqlitePool,
    entity_type: &str,
    entity_id: &str,
    user_id: &str,
) -> Result<Vec<EntityRelationRow>, DbError> {
    sqlx::query_as::<_, EntityRelationRow>(&format!(
        "SELECT {RELATION_COLS} FROM entity_relations \
         WHERE ((from_type = ? AND from_id = ?) OR (to_type = ? AND to_id = ?)) \
           AND user_id = ? \
         ORDER BY created_at ASC"
    ))
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
    sqlx::query_as::<_, EntityRelationRow>(&format!(
        "INSERT INTO entity_relations \
           (id, from_type, from_id, to_type, to_id, relation_type, user_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?) \
         RETURNING {RELATION_COLS}"
    ))
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
    use crate::test_helpers::{create_test_item, create_test_list, create_test_user, test_pool};
    use uuid::Uuid;

    #[tokio::test]
    async fn insert_returns_relation() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &user_id).await;
        let item_a = create_test_item(&pool, &list_id).await;
        let item_b = create_test_item(&pool, &list_id).await;
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
        let list_id = create_test_list(&pool, &user_id).await;
        let item_a = create_test_item(&pool, &list_id).await;
        let item_b = create_test_item(&pool, &list_id).await;
        let item_c = create_test_item(&pool, &list_id).await;

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
        let list_id = create_test_list(&pool, &user_id).await;
        let blocker = create_test_item(&pool, &list_id).await;
        let blocked = create_test_item(&pool, &list_id).await;

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
    async fn list_for_entity_with_titles_resolves_item_titles_and_completion() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &user_id).await;
        let item_a = create_test_item(&pool, &list_id).await;
        let item_b = create_test_item(&pool, &list_id).await;

        insert(
            &pool,
            InsertRelationInput {
                id: &Uuid::new_v4().to_string(),
                from_type: "item",
                from_id: &item_b,
                to_type: "item",
                to_id: &item_a,
                relation_type: "blocks",
                user_id: &user_id,
            },
        )
        .await
        .unwrap();

        let rows = list_for_entity_with_titles(&pool, "item", &item_a, &user_id)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].other_type, "item");
        assert_eq!(rows[0].other_title.as_deref(), Some("Test Item"));
        assert_eq!(rows[0].other_completed, Some(false));

        // The other side's completion is reported.
        sqlx::query("UPDATE items SET completed = 1 WHERE id = ?")
            .bind(&item_b)
            .execute(&pool)
            .await
            .unwrap();
        let rows = list_for_entity_with_titles(&pool, "item", &item_a, &user_id)
            .await
            .unwrap();
        assert_eq!(rows[0].other_completed, Some(true));

        // From the other side, the completed flag belongs to item_a.
        let rows = list_for_entity_with_titles(&pool, "item", &item_b, &user_id)
            .await
            .unwrap();
        assert_eq!(rows[0].other_completed, Some(false));
    }

    #[tokio::test]
    async fn list_for_entity_with_titles_resolves_list_and_container_names() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &user_id).await;
        let item_id = create_test_item(&pool, &list_id).await;
        let container_id = crate::test_helpers::create_test_container(&pool, &user_id, None).await;

        for (to_type, to_id) in [("list", &list_id), ("container", &container_id)] {
            insert(
                &pool,
                InsertRelationInput {
                    id: &Uuid::new_v4().to_string(),
                    from_type: "item",
                    from_id: &item_id,
                    to_type,
                    to_id,
                    relation_type: "relates_to",
                    user_id: &user_id,
                },
            )
            .await
            .unwrap();
        }

        let rows = list_for_entity_with_titles(&pool, "item", &item_id, &user_id)
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
        let list_row = rows.iter().find(|r| r.other_type == "list").unwrap();
        assert_eq!(list_row.other_title.as_deref(), Some("Test List"));
        assert!(list_row.other_completed.is_none());
        let container_row = rows.iter().find(|r| r.other_type == "container").unwrap();
        assert_eq!(container_row.other_title.as_deref(), Some("Test Container"));
    }

    #[tokio::test]
    async fn delete_removes_relation() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &user_id).await;
        let item_a = create_test_item(&pool, &list_id).await;
        let item_b = create_test_item(&pool, &list_id).await;
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
        let list_id = create_test_list(&pool, &user_id).await;
        let item_a = create_test_item(&pool, &list_id).await;
        let item_b = create_test_item(&pool, &list_id).await;
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
