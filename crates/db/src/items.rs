use crate::{
    DbError,
    types::{ContainerItemRow, ItemRow},
};
use sqlx::{Sqlite, SqliteConnection, SqlitePool};

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
    pub location_id: Option<Option<String>>,
}

/// One item rendered into an iCal feed, with its list name for the summary/description.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CalendarFeedItemRow {
    pub id: String,
    pub list_id: String,
    pub list_name: String,
    pub title: String,
    pub description: Option<String>,
    pub completed: bool,
    pub start_date: Option<String>,
    pub start_time: Option<String>,
    pub deadline: Option<String>,
    pub deadline_time: Option<String>,
    pub hard_deadline: Option<String>,
    pub updated_at: String,
}

// ── Column list constant ──────────────────────────────────────────────────────

const ITEM_COLUMNS: &str = "i.id, i.list_id, i.title, i.description, i.completed, i.position, \
     i.quantity, i.actual_quantity, i.unit, i.start_date, i.start_time, \
     i.deadline, i.deadline_time, i.hard_deadline, i.estimated_duration, \
     i.location_id, i.created_at, i.updated_at";

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

/// Returns items with `deadline < before_date` and `completed = 0`,
/// ordered by deadline ascending (earliest-overdue first).
#[tracing::instrument(skip(pool))]
pub async fn overdue(
    pool: &SqlitePool,
    user_id: &str,
    before_date: &str,
) -> Result<Vec<ItemRow>, DbError> {
    sqlx::query_as::<_, ItemRow>(&format!(
        "SELECT {ITEM_COLUMNS} FROM items i \
         JOIN lists l ON l.id = i.list_id \
         WHERE l.user_id = ? \
           AND i.deadline < ? \
           AND i.completed = 0 \
         ORDER BY i.deadline ASC NULLS LAST, i.position"
    ))
    .bind(user_id)
    .bind(before_date)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn list_all_for_user(pool: &SqlitePool, user_id: &str) -> Result<Vec<ItemRow>, DbError> {
    sqlx::query_as::<_, ItemRow>(&format!(
        "SELECT {ITEM_COLUMNS} FROM items i \
         JOIN lists l ON l.id = i.list_id \
         WHERE l.user_id = ? AND l.archived = 0 \
         ORDER BY l.position, i.position"
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Items for the iCal feed: all of the user's dated items (archived lists excluded),
/// optionally narrowed to a single list plus its direct sublists, with at least one date
/// on or after `from_date`. Completed items are included so subscribers see the whole
/// picture.
#[tracing::instrument(skip(pool))]
pub async fn for_calendar_feed(
    pool: &SqlitePool,
    user_id: &str,
    list_id: Option<&str>,
    from_date: &str,
) -> Result<Vec<CalendarFeedItemRow>, DbError> {
    sqlx::query_as::<_, CalendarFeedItemRow>(
        "SELECT i.id, i.list_id, l.name AS list_name, i.title, i.description, i.completed, \
         i.start_date, i.start_time, i.deadline, i.deadline_time, i.hard_deadline, i.updated_at \
         FROM items i \
         JOIN lists l ON l.id = i.list_id \
         WHERE l.user_id = ? AND l.archived = 0 \
           AND (? IS NULL OR i.list_id = ? OR l.parent_list_id = ?) \
           AND ( \
                (i.start_date IS NOT NULL AND i.start_date >= ?) \
             OR (i.deadline IS NOT NULL AND i.deadline >= ?) \
             OR (i.hard_deadline IS NOT NULL AND i.hard_deadline >= ?) \
           ) \
         ORDER BY COALESCE(i.start_date, i.deadline, i.hard_deadline), i.position",
    )
    .bind(user_id)
    .bind(list_id)
    .bind(list_id)
    .bind(list_id)
    .bind(from_date)
    .bind(from_date)
    .bind(from_date)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

pub async fn find_id_by_title_in_list(
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
    list_id: &str,
    title: &str,
    exclude_id: Option<&str>,
) -> Result<Option<String>, DbError> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM items \
         WHERE list_id = ? \
           AND LOWER(TRIM(title)) = LOWER(TRIM(?)) \
           AND id != COALESCE(?, '')",
    )
    .bind(list_id)
    .bind(title)
    .bind(exclude_id)
    .fetch_optional(executor)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(row.map(|(id,)| id))
}

// ── Write queries ─────────────────────────────────────────────────────────────

/// Columns written by every item INSERT. A new item column only needs an entry
/// here (plus the matching `InsertItemInput` field and bind in [`insert_row`]).
const ITEM_INSERT_COLUMNS: &str = "id, list_id, position, title, description, quantity, \
     actual_quantity, unit, start_date, start_time, deadline, deadline_time, \
     hard_deadline, estimated_duration";

/// The single item INSERT used by every write path.
///
/// The placeholders are derived from [`ITEM_INSERT_COLUMNS`] and the returned
/// columns from [`ITEM_COLUMNS`] (without the `i.` alias, which `RETURNING`
/// does not accept), so neither list is spelled out twice.
async fn insert_row<'e, E>(executor: E, input: &InsertItemInput) -> Result<ItemRow, DbError>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
    let placeholders = vec!["?"; ITEM_INSERT_COLUMNS.split(',').count()].join(", ");
    let returning = ITEM_COLUMNS.replace("i.", "");
    sqlx::query_as::<_, ItemRow>(&format!(
        "INSERT INTO items ({ITEM_INSERT_COLUMNS}) VALUES ({placeholders}) RETURNING {returning}"
    ))
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
    .fetch_one(executor)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn insert(pool: &SqlitePool, input: &InsertItemInput) -> Result<ItemRow, DbError> {
    let mut conn = pool.acquire().await.map_err(DbError::Sqlx)?;
    insert_row(&mut *conn, input).await
}

/// Insert multiple items through the shared INSERT.
/// The caller provides the transaction, so the whole batch stays atomic.
#[tracing::instrument(skip(conn, inputs), fields(count = inputs.len()))]
pub async fn insert_many_in_tx(
    conn: &mut SqliteConnection,
    inputs: &[InsertItemInput],
) -> Result<(), DbError> {
    for input in inputs {
        insert_row(&mut *conn, input).await?;
    }
    Ok(())
}

/// Same INSERT as `insert` but operates on an existing transaction connection.
#[tracing::instrument(skip(conn))]
pub async fn insert_in_tx(
    conn: &mut SqliteConnection,
    input: &InsertItemInput,
) -> Result<(), DbError> {
    insert_row(&mut *conn, input).await?;
    Ok(())
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
            location_id = CASE WHEN ? = 1 THEN ? ELSE location_id END, \
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
    .bind(input.location_id.is_some() as i32)
    .bind(input.location_id.as_ref().and_then(|v| v.as_deref()))
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
pub async fn toggle_complete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
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

/// Shared `move_item` statement so both the pool and the transaction variant
/// stay identical.
///
/// NOTE: target list ownership is NOT checked here — caller must verify that
/// `target_list_id` belongs to `user_id` before calling this function.
async fn move_item_row<'e, E>(
    executor: E,
    id: &str,
    user_id: &str,
    position: i32,
    target_list_id: Option<&str>,
) -> Result<bool, DbError>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
    let rows = sqlx::query(
        "UPDATE items SET position = ?, list_id = COALESCE(?, list_id), \
                          updated_at = datetime('now') \
         WHERE id = ? AND list_id IN (SELECT id FROM lists WHERE user_id = ?)",
    )
    .bind(position)
    .bind(target_list_id)
    .bind(id)
    .bind(user_id)
    .execute(executor)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

/// NOTE: target list ownership is NOT checked here — caller must verify that
/// `target_list_id` belongs to `user_id` before calling this function.
#[tracing::instrument(skip(pool))]
pub async fn move_item(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    position: i32,
    target_list_id: Option<&str>,
) -> Result<bool, DbError> {
    move_item_row(pool, id, user_id, position, target_list_id).await
}

/// Same as [`move_item`] but runs on a caller-owned transaction connection.
#[tracing::instrument(skip(conn))]
pub async fn move_item_in_tx(
    conn: &mut SqliteConnection,
    id: &str,
    user_id: &str,
    position: i32,
    target_list_id: Option<&str>,
) -> Result<bool, DbError> {
    move_item_row(&mut *conn, id, user_id, position, target_list_id).await
}

/// Rewrite the position of one item, but only if it currently lives in
/// `list_id` and that list belongs to `user_id`. Returns `false` when the item
/// is not in scope, so callers can abort instead of silently skipping it.
async fn set_position_row<'e, E>(
    executor: E,
    id: &str,
    user_id: &str,
    list_id: &str,
    position: i32,
) -> Result<bool, DbError>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
{
    let rows = sqlx::query(
        "UPDATE items SET position = ?, updated_at = datetime('now') \
         WHERE id = ? AND list_id = ? \
           AND list_id IN (SELECT id FROM lists WHERE user_id = ?)",
    )
    .bind(position)
    .bind(id)
    .bind(list_id)
    .bind(user_id)
    .execute(executor)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

/// See [`set_position_row`]; pool variant.
#[tracing::instrument(skip(pool))]
pub async fn set_position(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    list_id: &str,
    position: i32,
) -> Result<bool, DbError> {
    set_position_row(pool, id, user_id, list_id, position).await
}

/// See [`set_position_row`]; transaction variant.
#[tracing::instrument(skip(conn))]
pub async fn set_position_in_tx(
    conn: &mut SqliteConnection,
    id: &str,
    user_id: &str,
    list_id: &str,
    position: i32,
) -> Result<bool, DbError> {
    set_position_row(&mut *conn, id, user_id, list_id, position).await
}

/// All item ids of `list_id` owned by `user_id`, ordered by position.
/// Used to validate a reorder request against the real scope before writing.
pub async fn ordered_ids_in_list(
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
    user_id: &str,
    list_id: &str,
) -> Result<Vec<String>, DbError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT id FROM items WHERE list_id = ? \
           AND list_id IN (SELECT id FROM lists WHERE user_id = ?) \
         ORDER BY position, id",
    )
    .bind(list_id)
    .bind(user_id)
    .fetch_all(executor)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

#[tracing::instrument(skip(pool))]
pub async fn list_for_container(
    pool: &SqlitePool,
    container_id: &str,
    user_id: &str,
    limit: u32,
    offset: usize,
    recursive: bool,
) -> Result<Vec<ContainerItemRow>, DbError> {
    if recursive {
        sqlx::query_as::<_, ContainerItemRow>(&format!(
            "WITH RECURSIVE sub AS ( \
               SELECT id, name, printf('%06d', position) AS sort_path \
               FROM containers WHERE id = ? AND user_id = ? \
               UNION ALL \
               SELECT c.id, c.name, sub.sort_path || '/' || printf('%06d', c.position) \
               FROM containers c JOIN sub ON c.parent_container_id = sub.id \
               WHERE c.user_id = ? \
             ) \
             SELECT {ITEM_COLUMNS}, l.name AS list_name, l.container_id, sub.name AS container_name \
             FROM items i \
             JOIN lists l ON l.id = i.list_id \
             JOIN sub ON sub.id = l.container_id \
             WHERE l.user_id = ? AND l.archived = 0 \
             ORDER BY sub.sort_path, l.position, i.position \
             LIMIT ? OFFSET ?"
        ))
        .bind(container_id)
        .bind(user_id)
        .bind(user_id) // RECURSIVE sub: c.user_id filter
        .bind(user_id) // outer WHERE l.user_id
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    } else {
        sqlx::query_as::<_, ContainerItemRow>(&format!(
            "SELECT {ITEM_COLUMNS}, l.name AS list_name, l.container_id, c.name AS container_name \
             FROM items i \
             JOIN lists l ON l.id = i.list_id \
             JOIN containers c ON c.id = l.container_id \
             WHERE l.container_id = ? AND l.user_id = ? AND l.archived = 0 \
             ORDER BY l.position, i.position \
             LIMIT ? OFFSET ?"
        ))
        .bind(container_id)
        .bind(user_id)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(pool)
        .await
        .map_err(DbError::Sqlx)
    }
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
                location_id: None,
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
                location_id: None,
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

    #[tokio::test]
    async fn list_all_for_user_returns_items_across_lists() {
        let pool = crate::test_helpers::test_pool().await;
        let uid = crate::test_helpers::create_test_user(&pool).await;

        let lid1 = uuid::Uuid::new_v4().to_string();
        let lid2 = uuid::Uuid::new_v4().to_string();
        {
            let mut conn = pool.acquire().await.unwrap();
            crate::lists::insert(
                &mut conn,
                &crate::lists::InsertListInput {
                    id: lid1.clone(),
                    user_id: uid.clone(),
                    position: 0,
                    name: "L1".to_owned(),
                    list_type: "checklist".to_owned(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            crate::lists::insert(
                &mut conn,
                &crate::lists::InsertListInput {
                    id: lid2.clone(),
                    user_id: uid.clone(),
                    position: 1,
                    name: "L2".to_owned(),
                    list_type: "checklist".to_owned(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        }

        insert(
            &pool,
            &InsertItemInput {
                id: uuid::Uuid::new_v4().to_string(),
                list_id: lid1,
                position: 0,
                title: "A".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        insert(
            &pool,
            &InsertItemInput {
                id: uuid::Uuid::new_v4().to_string(),
                list_id: lid2,
                position: 0,
                title: "B".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let items = list_all_for_user(&pool, &uid).await.unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn list_all_for_user_excludes_other_users() {
        let pool = crate::test_helpers::test_pool().await;
        let uid1 = crate::test_helpers::create_test_user(&pool).await;
        let uid2 = crate::test_helpers::create_test_user(&pool).await;
        let lid = uuid::Uuid::new_v4().to_string();
        {
            let mut conn = pool.acquire().await.unwrap();
            crate::lists::insert(
                &mut conn,
                &crate::lists::InsertListInput {
                    id: lid.clone(),
                    user_id: uid1.clone(),
                    position: 0,
                    name: "L".to_owned(),
                    list_type: "checklist".to_owned(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        }
        insert(
            &pool,
            &InsertItemInput {
                id: uuid::Uuid::new_v4().to_string(),
                list_id: lid,
                position: 0,
                title: "Secret".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let items = list_all_for_user(&pool, &uid2).await.unwrap();
        assert!(items.is_empty(), "should not see other user's items");
    }

    #[tokio::test]
    async fn insert_many_in_tx_inserts_all_in_order() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &uid).await;

        let inputs: Vec<InsertItemInput> = (0..5)
            .map(|i| InsertItemInput {
                id: uuid::Uuid::new_v4().to_string(),
                list_id: list_id.clone(),
                position: i,
                title: format!("Item {i}"),
                ..Default::default()
            })
            .collect();

        let mut tx = pool.begin().await.unwrap();
        insert_many_in_tx(&mut tx, &inputs).await.unwrap();
        tx.commit().await.unwrap();

        let rows = list_for_list(&pool, &list_id, &uid).await.unwrap();
        assert_eq!(rows.len(), 5);
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.title, format!("Item {i}"));
            assert_eq!(row.position, i as i32);
        }
    }

    #[tokio::test]
    async fn insert_many_in_tx_empty_is_noop() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &uid).await;

        let mut tx = pool.begin().await.unwrap();
        insert_many_in_tx(&mut tx, &[]).await.unwrap();
        tx.commit().await.unwrap();

        let rows = list_for_list(&pool, &list_id, &uid).await.unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn find_by_title_exact_match() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &uid).await;
        let item = make_item(&pool, &list_id, "Buy milk", 0).await;
        let found = find_id_by_title_in_list(&pool, &list_id, "Buy milk", None)
            .await
            .unwrap();
        assert_eq!(found, Some(item.id));
    }

    #[tokio::test]
    async fn find_by_title_case_insensitive() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &uid).await;
        let item = make_item(&pool, &list_id, "Buy Milk", 0).await;
        let found = find_id_by_title_in_list(&pool, &list_id, "buy milk", None)
            .await
            .unwrap();
        assert_eq!(found, Some(item.id));
    }

    #[tokio::test]
    async fn find_by_title_excludes_self() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &uid).await;
        let item = make_item(&pool, &list_id, "Buy milk", 0).await;
        let found = find_id_by_title_in_list(&pool, &list_id, "Buy milk", Some(&item.id))
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn find_by_title_different_list_not_found() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list1 = setup_list(&pool, &uid).await;
        let list2 = setup_list(&pool, &uid).await;
        make_item(&pool, &list1, "Buy milk", 0).await;
        let found = find_id_by_title_in_list(&pool, &list2, "Buy milk", None)
            .await
            .unwrap();
        assert!(found.is_none());
    }

    // ── for_calendar_feed ─────────────────────────────────────────────────────

    /// Insert an item with optional dates; `pos` defaults to 0.
    async fn make_dated_item(
        pool: &SqlitePool,
        list_id: &str,
        title: &str,
        start_date: Option<&str>,
        deadline: Option<&str>,
        hard_deadline: Option<&str>,
    ) -> ItemRow {
        insert(
            pool,
            &InsertItemInput {
                id: uuid::Uuid::new_v4().to_string(),
                list_id: list_id.to_string(),
                position: 0,
                title: title.to_string(),
                start_date: start_date.map(str::to_string),
                deadline: deadline.map(str::to_string),
                hard_deadline: hard_deadline.map(str::to_string),
                ..Default::default()
            },
        )
        .await
        .unwrap()
    }

    async fn setup_named_list(pool: &SqlitePool, user_id: &str, name: &str) -> String {
        let list_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES (?, ?, ?)")
            .bind(&list_id)
            .bind(user_id)
            .bind(name)
            .execute(pool)
            .await
            .unwrap();
        list_id
    }

    #[tokio::test]
    async fn feed_includes_item_fields_and_list_name() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = setup_named_list(&pool, &uid, "Groceries").await;
        let item =
            make_dated_item(&pool, &list_id, "Buy milk", Some("2026-05-01"), None, None).await;

        let rows = for_calendar_feed(&pool, &uid, None, "2026-01-01")
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.id, item.id);
        assert_eq!(row.list_id, list_id);
        assert_eq!(row.list_name, "Groceries");
        assert_eq!(row.title, "Buy milk");
        assert!(!row.completed);
        assert_eq!(row.start_date.as_deref(), Some("2026-05-01"));
        assert!(row.deadline.is_none());
        assert!(!row.updated_at.is_empty());
    }

    #[tokio::test]
    async fn feed_scopes_to_user() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &uid).await;
        let other_list = setup_list(&pool, &other).await;
        make_dated_item(&pool, &list_id, "Mine", None, Some("2026-05-01"), None).await;
        make_dated_item(&pool, &other_list, "Theirs", None, Some("2026-05-01"), None).await;

        let rows = for_calendar_feed(&pool, &uid, None, "2026-01-01")
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "Mine");
    }

    #[tokio::test]
    async fn feed_scopes_to_list() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list1 = setup_list(&pool, &uid).await;
        let list2 = setup_list(&pool, &uid).await;
        make_dated_item(&pool, &list1, "In list 1", None, Some("2026-05-01"), None).await;
        make_dated_item(&pool, &list2, "In list 2", None, Some("2026-05-02"), None).await;

        let rows = for_calendar_feed(&pool, &uid, Some(&list1), "2026-01-01")
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "In list 1");
    }

    #[tokio::test]
    async fn feed_list_scope_includes_direct_sublists() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let parent = setup_list(&pool, &uid).await;
        let child = setup_named_list(&pool, &uid, "Child").await;
        sqlx::query("UPDATE lists SET parent_list_id = ? WHERE id = ?")
            .bind(&parent)
            .bind(&child)
            .execute(&pool)
            .await
            .unwrap();
        let grandchild = setup_named_list(&pool, &uid, "Grandchild").await;
        sqlx::query("UPDATE lists SET parent_list_id = ? WHERE id = ?")
            .bind(&child)
            .bind(&grandchild)
            .execute(&pool)
            .await
            .unwrap();
        let unrelated = setup_list(&pool, &uid).await;

        make_dated_item(&pool, &parent, "In parent", None, Some("2026-05-01"), None).await;
        make_dated_item(&pool, &child, "In child", None, Some("2026-05-02"), None).await;
        make_dated_item(
            &pool,
            &grandchild,
            "In grandchild",
            None,
            Some("2026-05-03"),
            None,
        )
        .await;
        make_dated_item(
            &pool,
            &unrelated,
            "Unrelated",
            None,
            Some("2026-05-04"),
            None,
        )
        .await;

        let rows = for_calendar_feed(&pool, &uid, Some(&parent), "2026-01-01")
            .await
            .unwrap();
        let titles: Vec<&str> = rows.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(titles, vec!["In parent", "In child"]);
    }

    #[tokio::test]
    async fn feed_filters_by_from_date() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &uid).await;
        make_dated_item(&pool, &list_id, "Old", None, Some("2025-12-31"), None).await;
        make_dated_item(
            &pool,
            &list_id,
            "On boundary",
            None,
            Some("2026-01-01"),
            None,
        )
        .await;
        make_dated_item(&pool, &list_id, "Future", None, Some("2026-06-01"), None).await;

        let rows = for_calendar_feed(&pool, &uid, None, "2026-01-01")
            .await
            .unwrap();
        let titles: Vec<&str> = rows.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(titles, vec!["On boundary", "Future"]);
    }

    #[tokio::test]
    async fn feed_matches_any_of_the_three_dates() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &uid).await;
        make_dated_item(&pool, &list_id, "By start", Some("2026-03-01"), None, None).await;
        make_dated_item(
            &pool,
            &list_id,
            "By deadline",
            None,
            Some("2026-03-02"),
            None,
        )
        .await;
        make_dated_item(&pool, &list_id, "By hard", None, None, Some("2026-03-03")).await;

        let rows = for_calendar_feed(&pool, &uid, None, "2026-01-01")
            .await
            .unwrap();
        let titles: Vec<&str> = rows.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(titles, vec!["By start", "By deadline", "By hard"]);
    }

    #[tokio::test]
    async fn feed_excludes_items_without_dates() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &uid).await;
        make_item(&pool, &list_id, "Undated", 0).await;
        make_dated_item(&pool, &list_id, "Dated", None, Some("2026-05-01"), None).await;

        let rows = for_calendar_feed(&pool, &uid, None, "2026-01-01")
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "Dated");
    }

    #[tokio::test]
    async fn feed_includes_completed_items() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &uid).await;
        let item = make_dated_item(&pool, &list_id, "Done", None, Some("2026-05-01"), None).await;
        set_completed(&pool, &item.id, true).await.unwrap();

        let rows = for_calendar_feed(&pool, &uid, None, "2026-01-01")
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].completed);
    }

    #[tokio::test]
    async fn feed_excludes_archived_lists() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &uid).await;
        let archived_id = setup_list(&pool, &uid).await;
        sqlx::query("UPDATE lists SET archived = 1 WHERE id = ?")
            .bind(&archived_id)
            .execute(&pool)
            .await
            .unwrap();
        make_dated_item(&pool, &list_id, "Active", None, Some("2026-05-01"), None).await;
        make_dated_item(
            &pool,
            &archived_id,
            "Archived",
            None,
            Some("2026-05-02"),
            None,
        )
        .await;

        let rows = for_calendar_feed(&pool, &uid, None, "2026-01-01")
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "Active");
    }

    #[tokio::test]
    async fn feed_ordered_by_earliest_date_then_position() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = setup_list(&pool, &uid).await;
        make_dated_item(&pool, &list_id, "Later", None, None, Some("2026-09-01")).await;
        make_dated_item(&pool, &list_id, "Sooner", Some("2026-02-01"), None, None).await;
        make_dated_item(&pool, &list_id, "Middle", None, Some("2026-04-01"), None).await;

        let rows = for_calendar_feed(&pool, &uid, None, "2026-01-01")
            .await
            .unwrap();
        let titles: Vec<&str> = rows.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(titles, vec!["Sooner", "Middle", "Later"]);
    }
}
