use crate::{types::ItemRow, DbError};
use sqlx::SqlitePool;

// ── Input types ───────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct InsertItemInput {
    pub id: String,
    pub list_id: String,
    pub position: i32,
    pub title: String,
    pub description: Option<String>,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<String>,
    pub start_time: Option<String>,
    pub deadline: Option<String>,
    pub deadline_time: Option<String>,
    pub hard_deadline: Option<String>,
    pub estimated_duration: Option<i32>,
}

#[derive(Debug)]
pub struct UpdateItemInput {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub completed: Option<bool>,
    pub quantity: Option<Option<i32>>,
    pub actual_quantity: Option<Option<i32>>,
    pub unit: Option<Option<String>>,
    pub start_date: Option<Option<String>>,
    pub start_time: Option<Option<String>>,
    pub deadline: Option<Option<String>>,
    pub deadline_time: Option<Option<String>>,
    pub hard_deadline: Option<Option<String>>,
    pub estimated_duration: Option<Option<i32>>,
}

// ── Column list constant ──────────────────────────────────────────────────────

const ITEM_COLUMNS: &str = "i.id, i.list_id, i.title, i.description, i.completed, i.position, \
     i.quantity, i.actual_quantity, i.unit, i.start_date, i.start_time, \
     i.deadline, i.deadline_time, i.hard_deadline, i.estimated_duration, \
     i.created_at, i.updated_at";

// ── Read queries ──────────────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn list_for_list(
    pool: &SqlitePool,
    list_id: &str,
    user_id: &str,
) -> Result<Vec<ItemRow>, DbError> {
    sqlx::query_as::<_, ItemRow>(&format!(
        "SELECT {ITEM_COLUMNS} FROM items i \
         JOIN lists l ON l.id = i.list_id \
         WHERE i.list_id = ? AND l.user_id = ? \
         ORDER BY i.position"
    ))
    .bind(list_id)
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
) -> Result<Option<ItemRow>, DbError> {
    sqlx::query_as::<_, ItemRow>(&format!(
        "SELECT {ITEM_COLUMNS} FROM items i \
         JOIN lists l ON l.id = i.list_id \
         WHERE i.id = ? AND l.user_id = ?"
    ))
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn by_date(
    pool: &SqlitePool,
    user_id: &str,
    date: &str,
) -> Result<Vec<ItemRow>, DbError> {
    sqlx::query_as::<_, ItemRow>(&format!(
        "SELECT {ITEM_COLUMNS} FROM items i \
         JOIN lists l ON l.id = i.list_id \
         WHERE l.user_id = ? \
           AND (i.start_date = ? OR i.deadline = ? OR i.hard_deadline = ?) \
         ORDER BY i.deadline ASC NULLS LAST, i.position"
    ))
    .bind(user_id)
    .bind(date)
    .bind(date)
    .bind(date)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn calendar(
    pool: &SqlitePool,
    user_id: &str,
    year_month: &str,
) -> Result<Vec<ItemRow>, DbError> {
    let prefix = format!("{year_month}%");
    sqlx::query_as::<_, ItemRow>(&format!(
        "SELECT {ITEM_COLUMNS} FROM items i \
         JOIN lists l ON l.id = i.list_id \
         WHERE l.user_id = ? \
           AND (i.start_date LIKE ? OR i.deadline LIKE ? OR i.hard_deadline LIKE ?) \
         ORDER BY i.start_date ASC NULLS LAST, i.deadline ASC NULLS LAST, i.position"
    ))
    .bind(user_id)
    .bind(&prefix)
    .bind(&prefix)
    .bind(&prefix)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

// ── Write queries ─────────────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn insert(pool: &SqlitePool, input: &InsertItemInput) -> Result<ItemRow, DbError> {
    sqlx::query_as::<_, ItemRow>(
        "INSERT INTO items (id, list_id, position, title, description, quantity, actual_quantity, \
                            unit, start_date, start_time, deadline, deadline_time, hard_deadline, \
                            estimated_duration) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
         RETURNING id, list_id, title, description, completed, position, \
                   quantity, actual_quantity, unit, start_date, start_time, \
                   deadline, deadline_time, hard_deadline, estimated_duration, \
                   created_at, updated_at",
    )
    .bind(&input.id)
    .bind(&input.list_id)
    .bind(input.position)
    .bind(&input.title)
    .bind(&input.description)
    .bind(input.quantity)
    .bind(input.actual_quantity)
    .bind(&input.unit)
    .bind(&input.start_date)
    .bind(&input.start_time)
    .bind(&input.deadline)
    .bind(&input.deadline_time)
    .bind(&input.hard_deadline)
    .bind(input.estimated_duration)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    input: &UpdateItemInput,
) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "UPDATE items SET \
            title = COALESCE(?, title), \
            description = CASE WHEN ? = 1 THEN ? ELSE description END, \
            completed = COALESCE(?, completed), \
            quantity = CASE WHEN ? = 1 THEN ? ELSE quantity END, \
            actual_quantity = CASE WHEN ? = 1 THEN ? ELSE actual_quantity END, \
            unit = CASE WHEN ? = 1 THEN ? ELSE unit END, \
            start_date = CASE WHEN ? = 1 THEN ? ELSE start_date END, \
            start_time = CASE WHEN ? = 1 THEN ? ELSE start_time END, \
            deadline = CASE WHEN ? = 1 THEN ? ELSE deadline END, \
            deadline_time = CASE WHEN ? = 1 THEN ? ELSE deadline_time END, \
            hard_deadline = CASE WHEN ? = 1 THEN ? ELSE hard_deadline END, \
            estimated_duration = CASE WHEN ? = 1 THEN ? ELSE estimated_duration END, \
            updated_at = datetime('now') \
         WHERE id = ? AND list_id IN (SELECT id FROM lists WHERE user_id = ?)",
    )
    .bind(input.title.as_deref())
    .bind(input.description.is_some() as i32)
    .bind(input.description.as_ref().and_then(|v| v.as_deref()))
    .bind(input.completed.map(|v| v as i32))
    .bind(input.quantity.is_some() as i32)
    .bind(input.quantity.and_then(|v| v))
    .bind(input.actual_quantity.is_some() as i32)
    .bind(input.actual_quantity.and_then(|v| v))
    .bind(input.unit.is_some() as i32)
    .bind(input.unit.as_ref().and_then(|v| v.as_deref()))
    .bind(input.start_date.is_some() as i32)
    .bind(input.start_date.as_ref().and_then(|v| v.as_deref()))
    .bind(input.start_time.is_some() as i32)
    .bind(input.start_time.as_ref().and_then(|v| v.as_deref()))
    .bind(input.deadline.is_some() as i32)
    .bind(input.deadline.as_ref().and_then(|v| v.as_deref()))
    .bind(input.deadline_time.is_some() as i32)
    .bind(input.deadline_time.as_ref().and_then(|v| v.as_deref()))
    .bind(input.hard_deadline.is_some() as i32)
    .bind(input.hard_deadline.as_ref().and_then(|v| v.as_deref()))
    .bind(input.estimated_duration.is_some() as i32)
    .bind(input.estimated_duration.and_then(|v| v))
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

/// Set completed status directly by item ID.
/// **Ownership is NOT checked** — caller must verify the item belongs to user_id
/// before calling this function. Only call after a successful `get_one` or `update`
/// that already enforced ownership.
#[tracing::instrument(skip(pool))]
pub async fn set_completed(pool: &SqlitePool, id: &str, completed: bool) -> Result<(), DbError> {
    sqlx::query("UPDATE items SET completed = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(completed as i32)
        .bind(id)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(())
}

#[tracing::instrument(skip(pool))]
pub async fn toggle_complete(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "UPDATE items SET completed = 1 - completed, updated_at = datetime('now') \
         WHERE id = ? AND list_id IN (SELECT id FROM lists WHERE user_id = ?)",
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "DELETE FROM items WHERE id = ? AND list_id IN (SELECT id FROM lists WHERE user_id = ?)",
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn move_item(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    position: i64,
    target_list_id: Option<&str>,
) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "UPDATE items SET position = ?, list_id = COALESCE(?, list_id), \
                          updated_at = datetime('now') \
         WHERE id = ? AND list_id IN (SELECT id FROM lists WHERE user_id = ?)",
    )
    .bind(position)
    .bind(target_list_id)
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
    use sqlx::SqlitePool;

    async fn setup_list(pool: &SqlitePool, user_id: &str) -> String {
        let list_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES (?, ?, 'Test List')")
            .bind(&list_id)
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
        list_id
    }

    async fn make_item(pool: &SqlitePool, list_id: &str, title: &str, pos: i32) -> ItemRow {
        insert(
            pool,
            &InsertItemInput {
                id: uuid::Uuid::new_v4().to_string(),
                list_id: list_id.to_string(),
                position: pos,
                title: title.to_string(),
                ..Default::default()
            },
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn insert_returning_gives_item() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &user_id).await;

        let item = insert(
            &pool,
            &InsertItemInput {
                id: "item-1".to_string(),
                list_id: list_id.clone(),
                position: 0,
                title: "Buy milk".to_string(),
                description: Some("whole milk".to_string()),
                quantity: Some(2),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        assert_eq!(item.id, "item-1");
        assert_eq!(item.list_id, list_id);
        assert_eq!(item.title, "Buy milk");
        assert_eq!(item.description.as_deref(), Some("whole milk"));
        assert!(!item.completed);
        assert_eq!(item.position, 0);
        assert_eq!(item.quantity, Some(2));
    }

    #[tokio::test]
    async fn list_for_list_ordered_by_position() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &user_id).await;

        make_item(&pool, &list_id, "Second", 1).await;
        make_item(&pool, &list_id, "First", 0).await;

        let items = list_for_list(&pool, &list_id, &user_id).await.unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "First");
        assert_eq!(items[1].title, "Second");
    }

    #[tokio::test]
    async fn get_one_not_found_returns_none() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;

        let result = get_one(&pool, "nonexistent-id", &user_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_one_wrong_user_returns_none() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let other_user = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &user_id).await;

        let item = make_item(&pool, &list_id, "My item", 0).await;

        let result = get_one(&pool, &item.id, &other_user).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn by_date_matches_deadline() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &user_id).await;

        let item_id = uuid::Uuid::new_v4().to_string();
        insert(
            &pool,
            &InsertItemInput {
                id: item_id.clone(),
                list_id: list_id.clone(),
                position: 0,
                title: "Deadline item".to_string(),
                deadline: Some("2026-05-15".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let items = by_date(&pool, &user_id, "2026-05-15").await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, item_id);

        let items_empty = by_date(&pool, &user_id, "2026-05-16").await.unwrap();
        assert!(items_empty.is_empty());

        // Also verify hard_deadline is matched
        let hard_item_id = uuid::Uuid::new_v4().to_string();
        insert(
            &pool,
            &InsertItemInput {
                id: hard_item_id.clone(),
                list_id: list_id.clone(),
                position: 1,
                title: "Hard deadline item".to_string(),
                hard_deadline: Some("2026-05-15".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let items_two = by_date(&pool, &user_id, "2026-05-15").await.unwrap();
        assert_eq!(items_two.len(), 2);
    }

    #[tokio::test]
    async fn calendar_matches_month_prefix() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &user_id).await;

        let item_id = uuid::Uuid::new_v4().to_string();
        insert(
            &pool,
            &InsertItemInput {
                id: item_id.clone(),
                list_id: list_id.clone(),
                position: 0,
                title: "May item".to_string(),
                start_date: Some("2026-05-10".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let items = calendar(&pool, &user_id, "2026-05").await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, item_id);

        let items_empty = calendar(&pool, &user_id, "2026-06").await.unwrap();
        assert!(items_empty.is_empty());

        // Also verify deadline column is matched
        let deadline_item_id = uuid::Uuid::new_v4().to_string();
        insert(
            &pool,
            &InsertItemInput {
                id: deadline_item_id.clone(),
                list_id: list_id.clone(),
                position: 1,
                title: "Deadline item".to_string(),
                deadline: Some("2026-05-20".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let items_two = calendar(&pool, &user_id, "2026-05").await.unwrap();
        assert_eq!(items_two.len(), 2);
    }

    #[tokio::test]
    async fn update_coalesce_only_changes_provided_fields() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &user_id).await;

        let item_id = uuid::Uuid::new_v4().to_string();
        let item = insert(
            &pool,
            &InsertItemInput {
                id: item_id.clone(),
                list_id: list_id.clone(),
                position: 0,
                title: "Original".to_string(),
                quantity: Some(5),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let changed = update(
            &pool,
            &item.id,
            &user_id,
            &UpdateItemInput {
                title: Some("Updated".to_string()),
                description: None,
                completed: None,
                quantity: None,
                actual_quantity: None,
                unit: None,
                start_date: None,
                start_time: None,
                deadline: None,
                deadline_time: None,
                hard_deadline: None,
                estimated_duration: None,
            },
        )
        .await
        .unwrap();
        assert!(changed);

        let updated = get_one(&pool, &item.id, &user_id).await.unwrap().unwrap();
        assert_eq!(updated.title, "Updated");
        assert!(!updated.completed);
        assert_eq!(updated.quantity, Some(5));
    }

    #[tokio::test]
    async fn update_nullable_field_to_none_clears_it() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &user_id).await;

        let item_id = uuid::Uuid::new_v4().to_string();
        insert(
            &pool,
            &InsertItemInput {
                id: item_id.clone(),
                list_id: list_id.clone(),
                position: 0,
                title: "Item".to_string(),
                description: Some("some description".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        update(
            &pool,
            &item_id,
            &user_id,
            &UpdateItemInput {
                title: None,
                description: Some(None),
                completed: None,
                quantity: None,
                actual_quantity: None,
                unit: None,
                start_date: None,
                start_time: None,
                deadline: None,
                deadline_time: None,
                hard_deadline: None,
                estimated_duration: None,
            },
        )
        .await
        .unwrap();

        let updated = get_one(&pool, &item_id, &user_id).await.unwrap().unwrap();
        assert!(updated.description.is_none());
    }

    #[tokio::test]
    async fn toggle_complete_flips() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &user_id).await;

        let item = make_item(&pool, &list_id, "Toggle me", 0).await;
        assert!(!item.completed);

        toggle_complete(&pool, &item.id, &user_id).await.unwrap();
        let after_first = get_one(&pool, &item.id, &user_id).await.unwrap().unwrap();
        assert!(after_first.completed);

        toggle_complete(&pool, &item.id, &user_id).await.unwrap();
        let after_second = get_one(&pool, &item.id, &user_id).await.unwrap().unwrap();
        assert!(!after_second.completed);
    }

    #[tokio::test]
    async fn delete_removes_item() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &user_id).await;

        let item = make_item(&pool, &list_id, "Doomed", 0).await;

        let deleted = delete(&pool, &item.id, &user_id).await.unwrap();
        assert!(deleted);

        let result = get_one(&pool, &item.id, &user_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn move_item_updates_position() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &user_id).await;

        let item = make_item(&pool, &list_id, "Move me", 0).await;
        assert_eq!(item.position, 0);

        let moved = move_item(&pool, &item.id, &user_id, 5, None).await.unwrap();
        assert!(moved);

        let updated = get_one(&pool, &item.id, &user_id).await.unwrap().unwrap();
        assert_eq!(updated.position, 5);
    }
}
