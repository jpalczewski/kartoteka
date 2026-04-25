use crate::{
    DbError,
    types::{TagRow, TemplateItemRow, TemplateRow},
};
use sqlx::{SqliteConnection, SqlitePool};

// ── Read ──────────────────────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn list_all(pool: &SqlitePool, user_id: &str) -> Result<Vec<TemplateRow>, DbError> {
    sqlx::query_as::<_, TemplateRow>(
        "SELECT id, user_id, name, icon, description, created_at \
         FROM templates WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn get_one(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<TemplateRow>, DbError> {
    sqlx::query_as::<_, TemplateRow>(
        "SELECT id, user_id, name, icon, description, created_at \
         FROM templates WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// OWNERSHIP NOT CHECKED — caller must verify the template belongs to `user_id` before calling.
#[tracing::instrument(skip(pool))]
pub async fn get_items(
    pool: &SqlitePool,
    template_id: &str,
) -> Result<Vec<TemplateItemRow>, DbError> {
    sqlx::query_as::<_, TemplateItemRow>(
        "SELECT id, template_id, title, description, position, quantity, unit, created_at \
         FROM template_items WHERE template_id = ? ORDER BY position",
    )
    .bind(template_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// OWNERSHIP NOT CHECKED — caller must verify the template belongs to `user_id` before calling.
#[tracing::instrument(skip(pool))]
pub async fn get_tags(pool: &SqlitePool, template_id: &str) -> Result<Vec<TagRow>, DbError> {
    sqlx::query_as::<_, TagRow>(
        "SELECT t.id, t.user_id, t.name, t.icon, t.color, t.parent_tag_id, \
                t.tag_type, t.metadata, t.created_at \
         FROM tags t \
         JOIN template_tags tt ON tt.tag_id = t.id \
         WHERE tt.template_id = ?",
    )
    .bind(template_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

// ── Write ─────────────────────────────────────────────────────────────────────

#[tracing::instrument(skip(conn))]
pub async fn insert(
    conn: &mut SqliteConnection,
    id: &str,
    user_id: &str,
    name: &str,
    icon: Option<&str>,
    description: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO templates (id, user_id, name, icon, description) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(user_id)
    .bind(name)
    .bind(icon)
    .bind(description)
    .execute(conn)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip(conn))]
pub async fn insert_item(
    conn: &mut SqliteConnection,
    id: &str,
    template_id: &str,
    title: &str,
    description: Option<&str>,
    position: i32,
    quantity: Option<i32>,
    unit: Option<&str>,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO template_items (id, template_id, title, description, position, quantity, unit) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(template_id)
    .bind(title)
    .bind(description)
    .bind(position)
    .bind(quantity)
    .bind(unit)
    .execute(conn)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(())
}

#[tracing::instrument(skip(conn))]
pub async fn insert_tag(
    conn: &mut SqliteConnection,
    template_id: &str,
    tag_id: &str,
) -> Result<(), DbError> {
    sqlx::query("INSERT OR IGNORE INTO template_tags (template_id, tag_id) VALUES (?, ?)")
        .bind(template_id)
        .bind(tag_id)
        .execute(conn)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(())
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let rows = sqlx::query("DELETE FROM templates WHERE id = ? AND user_id = ?")
        .bind(id)
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

    #[tokio::test]
    async fn insert_and_list() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let id = uuid::Uuid::new_v4().to_string();
        {
            let mut conn = pool.acquire().await.unwrap();
            insert(&mut conn, &id, &uid, "My Template", None, None)
                .await
                .unwrap();
        }
        let rows = list_all(&pool, &uid).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "My Template");
    }

    #[tokio::test]
    async fn get_one_returns_none_for_wrong_user() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let uid2 = create_test_user(&pool).await;
        let id = uuid::Uuid::new_v4().to_string();
        {
            let mut conn = pool.acquire().await.unwrap();
            insert(&mut conn, &id, &uid, "T", None, None).await.unwrap();
        }
        let row = get_one(&pool, &id, &uid2).await.unwrap();
        assert!(row.is_none());
    }

    #[tokio::test]
    async fn insert_items_and_get() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let tid = uuid::Uuid::new_v4().to_string();
        let iid = uuid::Uuid::new_v4().to_string();
        {
            let mut conn = pool.acquire().await.unwrap();
            insert(&mut conn, &tid, &uid, "T", None, None)
                .await
                .unwrap();
            insert_item(&mut conn, &iid, &tid, "Item A", None, 0, None, None)
                .await
                .unwrap();
        }
        let items = get_items(&pool, &tid).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Item A");
    }

    #[tokio::test]
    async fn delete_removes_template() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let id = uuid::Uuid::new_v4().to_string();
        {
            let mut conn = pool.acquire().await.unwrap();
            insert(&mut conn, &id, &uid, "T", None, None).await.unwrap();
        }
        let deleted = delete(&pool, &id, &uid).await.unwrap();
        assert!(deleted);
        assert!(list_all(&pool, &uid).await.unwrap().is_empty());
    }
}
