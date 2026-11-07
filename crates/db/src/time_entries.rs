use crate::{DbError, types::TimeEntryRow};
use sqlx::{QueryBuilder, Sqlite, SqlitePool};
use std::collections::HashMap;

// ── Constants ─────────────────────────────────────────────────────────────────

const TIME_ENTRY_COLS: &str =
    "id, item_id, user_id, description, started_at, ended_at, duration, source, mode, created_at";

/// Seconds contributed by a single `time_entries te` row.
///
/// A completed entry contributes its stored `duration`; a running entry
/// (`ended_at IS NULL`) contributes the seconds elapsed since `started_at`, so a
/// live project/item total is already correct before the timer is stopped.
/// Timestamps are stored as UTC `YYYY-MM-DD HH:MM:SS`, which SQLite parses as UTC.
const TRACKED_SECONDS: &str = "CASE WHEN te.ended_at IS NULL \
     THEN MAX(0, CAST(strftime('%s','now') AS INTEGER) - CAST(strftime('%s', te.started_at) AS INTEGER)) \
     ELSE COALESCE(te.duration, 0) END";

/// `started_at` of the running entry of a group, if the group has one.
const RUNNING_SINCE: &str = "MIN(CASE WHEN te.ended_at IS NULL THEN te.started_at END)";

// ── Aggregation types ────────────────────────────────────────────────────────

/// Tracked time for one entity (item, list subtree or container subtree).
#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct TimeTotalRow {
    /// Completed entries plus the live elapsed time of a running entry.
    pub seconds: i64,
    /// `started_at` of the running entry counted into `seconds`, if any.
    pub running_since: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct EntityTimeTotalRow {
    entity_id: String,
    seconds: i64,
    running_since: Option<String>,
}

impl From<EntityTimeTotalRow> for (String, TimeTotalRow) {
    fn from(r: EntityTimeTotalRow) -> Self {
        (
            r.entity_id,
            TimeTotalRow {
                seconds: r.seconds,
                running_since: r.running_since,
            },
        )
    }
}

// ── Read ──────────────────────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn get_one(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<TimeEntryRow>, DbError> {
    sqlx::query_as::<_, TimeEntryRow>(&format!(
        "SELECT {TIME_ENTRY_COLS} FROM time_entries WHERE id = ? AND user_id = ?"
    ))
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn get_running(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Option<TimeEntryRow>, DbError> {
    sqlx::query_as::<_, TimeEntryRow>(&format!(
        "SELECT {TIME_ENTRY_COLS} FROM time_entries WHERE user_id = ? AND ended_at IS NULL ORDER BY started_at ASC LIMIT 1"
    ))
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn list_for_item(
    pool: &SqlitePool,
    item_id: &str,
    user_id: &str,
) -> Result<Vec<TimeEntryRow>, DbError> {
    sqlx::query_as::<_, TimeEntryRow>(&format!(
        "SELECT {TIME_ENTRY_COLS} FROM time_entries WHERE item_id = ? AND user_id = ? ORDER BY created_at DESC"
    ))
    .bind(item_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn list_inbox(pool: &SqlitePool, user_id: &str) -> Result<Vec<TimeEntryRow>, DbError> {
    sqlx::query_as::<_, TimeEntryRow>(&format!(
        "SELECT {TIME_ENTRY_COLS} FROM time_entries WHERE user_id = ? AND item_id IS NULL ORDER BY created_at DESC"
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn list_all_for_user(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<TimeEntryRow>, DbError> {
    sqlx::query_as::<_, TimeEntryRow>(&format!(
        "SELECT {TIME_ENTRY_COLS} FROM time_entries WHERE user_id = ? ORDER BY created_at DESC LIMIT 200"
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Returns (total_seconds, entry_count) for completed entries on an item.
#[tracing::instrument(skip(pool))]
pub async fn summary_for_item(
    pool: &SqlitePool,
    item_id: &str,
    user_id: &str,
) -> Result<(i64, i64), DbError> {
    let row: (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(duration), 0), COUNT(*) FROM time_entries \
         WHERE item_id = ? AND user_id = ? AND ended_at IS NOT NULL",
    )
    .bind(item_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(row)
}

/// Tracked time for a single item. Inbox entries (`item_id IS NULL`) never match.
#[tracing::instrument(skip(pool))]
pub async fn item_total(
    pool: &SqlitePool,
    user_id: &str,
    item_id: &str,
) -> Result<TimeTotalRow, DbError> {
    sqlx::query_as::<_, TimeTotalRow>(&format!(
        "SELECT COALESCE(SUM({TRACKED_SECONDS}), 0) AS seconds, {RUNNING_SINCE} AS running_since \
         FROM time_entries te WHERE te.user_id = ? AND te.item_id = ?"
    ))
    .bind(user_id)
    .bind(item_id)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Tracked time for many items in one query, keyed by item id.
/// Items without entries are absent from the map (callers fill in zeros).
#[tracing::instrument(skip(pool))]
pub async fn item_totals(
    pool: &SqlitePool,
    user_id: &str,
    item_ids: &[String],
) -> Result<HashMap<String, TimeTotalRow>, DbError> {
    if item_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(format!(
        "SELECT te.item_id AS entity_id, COALESCE(SUM({TRACKED_SECONDS}), 0) AS seconds, \
         {RUNNING_SINCE} AS running_since FROM time_entries te WHERE te.user_id = "
    ));
    qb.push_bind(user_id);
    qb.push(" AND te.item_id IN (");
    {
        let mut sep = qb.separated(", ");
        for id in item_ids {
            sep.push_bind(id);
        }
    }
    qb.push(") GROUP BY te.item_id");
    let rows: Vec<EntityTimeTotalRow> = qb.build_query_as().fetch_all(pool).await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Tracked time for a list **including every sublist at any depth**.
#[tracing::instrument(skip(pool))]
pub async fn list_total(
    pool: &SqlitePool,
    user_id: &str,
    list_id: &str,
) -> Result<TimeTotalRow, DbError> {
    sqlx::query_as::<_, TimeTotalRow>(&format!(
        "WITH RECURSIVE subtree(id) AS ( \
           SELECT id FROM lists WHERE id = ? AND user_id = ? \
           UNION \
           SELECT l.id FROM lists l JOIN subtree s ON l.parent_list_id = s.id WHERE l.user_id = ? \
         ) \
         SELECT COALESCE(SUM({TRACKED_SECONDS}), 0) AS seconds, {RUNNING_SINCE} AS running_since \
         FROM time_entries te \
         JOIN items i ON i.id = te.item_id \
         WHERE te.user_id = ? AND i.list_id IN (SELECT id FROM subtree)"
    ))
    .bind(list_id)
    .bind(user_id)
    .bind(user_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Roll-up per list (each including its own sublists) for many lists at once.
/// Lists without entries are absent from the map (callers fill in zeros).
#[tracing::instrument(skip(pool))]
pub async fn list_totals(
    pool: &SqlitePool,
    user_id: &str,
    list_ids: &[String],
) -> Result<HashMap<String, TimeTotalRow>, DbError> {
    if list_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        "WITH RECURSIVE subtree(root_id, id) AS ( \
           SELECT id, id FROM lists WHERE user_id = ",
    );
    qb.push_bind(user_id);
    qb.push(" AND id IN (");
    {
        let mut sep = qb.separated(", ");
        for id in list_ids {
            sep.push_bind(id);
        }
    }
    qb.push(
        ") \
         UNION \
         SELECT s.root_id, l.id FROM lists l JOIN subtree s ON l.parent_list_id = s.id WHERE l.user_id = ",
    );
    qb.push_bind(user_id);
    qb.push(format!(
        ") \
         SELECT s.root_id AS entity_id, COALESCE(SUM({TRACKED_SECONDS}), 0) AS seconds, \
                {RUNNING_SINCE} AS running_since \
         FROM subtree s \
         JOIN items i ON i.list_id = s.id \
         JOIN time_entries te ON te.item_id = i.id AND te.user_id = "
    ));
    qb.push_bind(user_id);
    qb.push(" GROUP BY s.root_id");
    let rows: Vec<EntityTimeTotalRow> = qb.build_query_as().fetch_all(pool).await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Tracked time for a container **including all sub-containers at any depth**,
/// every list in that subtree and those lists' sublists.
#[tracing::instrument(skip(pool))]
pub async fn container_total(
    pool: &SqlitePool,
    user_id: &str,
    container_id: &str,
) -> Result<TimeTotalRow, DbError> {
    sqlx::query_as::<_, TimeTotalRow>(&format!(
        "WITH RECURSIVE sub(id) AS ( \
           SELECT id FROM containers WHERE id = ? AND user_id = ? \
           UNION \
           SELECT c.id FROM containers c JOIN sub s ON c.parent_container_id = s.id WHERE c.user_id = ? \
         ), \
         sub_lists(id) AS ( \
           SELECT l.id FROM lists l WHERE l.user_id = ? AND l.container_id IN (SELECT id FROM sub) \
           UNION \
           SELECT l.id FROM lists l JOIN sub_lists sl ON l.parent_list_id = sl.id WHERE l.user_id = ? \
         ) \
         SELECT COALESCE(SUM({TRACKED_SECONDS}), 0) AS seconds, {RUNNING_SINCE} AS running_since \
         FROM time_entries te \
         JOIN items i ON i.id = te.item_id \
         WHERE te.user_id = ? AND i.list_id IN (SELECT id FROM sub_lists)"
    ))
    .bind(container_id)
    .bind(user_id)
    .bind(user_id)
    .bind(user_id)
    .bind(user_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)
}

// ── Write ─────────────────────────────────────────────────────────────────────

pub struct InsertTimeEntryInput<'a> {
    pub id: &'a str,
    pub item_id: Option<&'a str>,
    pub user_id: &'a str,
    pub description: Option<&'a str>,
    pub started_at: &'a str,
    pub source: &'a str,
    pub mode: Option<&'a str>,
    pub ended_at: Option<&'a str>,
    pub duration: Option<i32>,
}

#[tracing::instrument(skip(pool, input), fields(item_id = ?input.item_id, source = %input.source))]
pub async fn insert(
    pool: &SqlitePool,
    input: InsertTimeEntryInput<'_>,
) -> Result<TimeEntryRow, DbError> {
    sqlx::query_as::<_, TimeEntryRow>(&format!(
        "INSERT INTO time_entries \
           (id, item_id, user_id, description, started_at, source, mode, ended_at, duration) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) \
         RETURNING {TIME_ENTRY_COLS}"
    ))
    .bind(input.id)
    .bind(input.item_id)
    .bind(input.user_id)
    .bind(input.description)
    .bind(input.started_at)
    .bind(input.source)
    .bind(input.mode)
    .bind(input.ended_at)
    .bind(input.duration)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn stop(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    ended_at: &str,
    duration: i32,
) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "UPDATE time_entries SET ended_at = ?, duration = ? WHERE id = ? AND user_id = ? AND ended_at IS NULL",
    )
    .bind(ended_at)
    .bind(duration)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn assign(
    pool: &SqlitePool,
    id: &str,
    item_id: &str,
    user_id: &str,
) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "UPDATE time_entries SET item_id = ? WHERE id = ? AND user_id = ? AND item_id IS NULL",
    )
    .bind(item_id)
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let rows = sqlx::query("DELETE FROM time_entries WHERE id = ? AND user_id = ?")
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
    use crate::test_helpers::{
        create_test_container, create_test_item, create_test_list, create_test_list_in_container,
        create_test_sublist, create_test_user, test_pool,
    };
    use uuid::Uuid;

    async fn make_entry(
        pool: &SqlitePool,
        user_id: &str,
        item_id: Option<&str>,
        ended_at: Option<&str>,
        duration: Option<i32>,
    ) -> TimeEntryRow {
        insert(
            pool,
            InsertTimeEntryInput {
                id: &Uuid::new_v4().to_string(),
                item_id,
                user_id,
                description: None,
                started_at: "2026-01-01 10:00:00",
                source: "timer",
                mode: Some("stopwatch"),
                ended_at,
                duration,
            },
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn insert_returns_entry() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;

        let entry = make_entry(&pool, &uid, Some(&item_id), None, None).await;
        assert_eq!(entry.user_id, uid);
        assert_eq!(entry.item_id.as_deref(), Some(item_id.as_str()));
        assert_eq!(entry.source, "timer");
        assert!(entry.ended_at.is_none());
        assert!(entry.duration.is_none());
    }

    #[tokio::test]
    async fn get_running_returns_active_entry() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        // No running entry yet
        assert!(get_running(&pool, &uid).await.unwrap().is_none());

        // Insert a running entry (ended_at = None)
        let entry = make_entry(&pool, &uid, None, None, None).await;
        let running = get_running(&pool, &uid).await.unwrap();
        assert!(running.is_some());
        assert_eq!(running.unwrap().id, entry.id);

        // Insert a completed entry — should not change result
        make_entry(&pool, &uid, None, Some("2026-01-01 11:00:00"), Some(3600)).await;
        let running = get_running(&pool, &uid).await.unwrap();
        assert!(running.is_some()); // still the first running one
    }

    #[tokio::test]
    async fn stop_sets_ended_at_and_duration() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let entry = make_entry(&pool, &uid, None, None, None).await;

        let affected = stop(&pool, &entry.id, &uid, "2026-01-01 11:00:00", 3600)
            .await
            .unwrap();
        assert!(affected);

        let updated = get_one(&pool, &entry.id, &uid).await.unwrap().unwrap();
        assert_eq!(updated.ended_at.as_deref(), Some("2026-01-01 11:00:00"));
        assert_eq!(updated.duration, Some(3600));

        // Running timer should now be gone
        assert!(get_running(&pool, &uid).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_inbox_returns_only_unassigned() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;

        make_entry(&pool, &uid, Some(&item_id), None, None).await; // assigned
        make_entry(&pool, &uid, None, None, None).await; // unassigned

        let inbox = list_inbox(&pool, &uid).await.unwrap();
        assert_eq!(inbox.len(), 1);
        assert!(inbox[0].item_id.is_none());
    }

    #[tokio::test]
    async fn list_for_item_returns_only_that_item() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_a = create_test_item(&pool, &list_id).await;
        let item_b = create_test_item(&pool, &list_id).await;

        make_entry(&pool, &uid, Some(&item_a), None, None).await;
        make_entry(&pool, &uid, Some(&item_b), None, None).await;

        let rows = list_for_item(&pool, &item_a, &uid).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].item_id.as_deref(), Some(item_a.as_str()));
    }

    #[tokio::test]
    async fn assign_changes_item_id() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;

        let entry = make_entry(&pool, &uid, None, None, None).await;
        assert!(entry.item_id.is_none());

        let affected = assign(&pool, &entry.id, &item_id, &uid).await.unwrap();
        assert!(affected);

        let updated = get_one(&pool, &entry.id, &uid).await.unwrap().unwrap();
        assert_eq!(updated.item_id.as_deref(), Some(item_id.as_str()));
    }

    #[tokio::test]
    async fn delete_removes_entry() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let entry = make_entry(&pool, &uid, None, None, None).await;

        let deleted = delete(&pool, &entry.id, &uid).await.unwrap();
        assert!(deleted);
        assert!(get_one(&pool, &entry.id, &uid).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_wrong_user_returns_false() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let entry = make_entry(&pool, &uid, None, None, None).await;

        let deleted = delete(&pool, &entry.id, &other).await.unwrap();
        assert!(!deleted);
        assert!(get_one(&pool, &entry.id, &uid).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn summary_for_item_sums_completed_entries() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;

        // Two completed entries (60s + 120s = 180s)
        make_entry(
            &pool,
            &uid,
            Some(&item_id),
            Some("2026-01-01 10:01:00"),
            Some(60),
        )
        .await;
        make_entry(
            &pool,
            &uid,
            Some(&item_id),
            Some("2026-01-01 10:03:00"),
            Some(120),
        )
        .await;
        // One running entry (no duration) — should NOT be counted
        make_entry(&pool, &uid, Some(&item_id), None, None).await;

        let (total, count) = summary_for_item(&pool, &item_id, &uid).await.unwrap();
        assert_eq!(total, 180);
        assert_eq!(count, 2);
    }

    // ── Recursive roll-up ─────────────────────────────────────────────────────

    /// Build a nested tree: container A > (container B > list B1 > (item b1, sublist B1s > item b1s)),
    /// plus list A1 directly in A (item a1).
    async fn nested_tree(pool: &SqlitePool, uid: &str) -> (String, SimpleTree) {
        let root = create_test_container(pool, uid, None).await;
        let child = create_test_container(pool, uid, Some(&root)).await;

        let list_a1 = create_test_list_in_container(pool, uid, &root).await;
        let item_a1 = create_test_item(pool, &list_a1).await;

        let list_b1 = create_test_list_in_container(pool, uid, &child).await;
        let item_b1 = create_test_item(pool, &list_b1).await;

        let sublist_b1s = create_test_sublist(pool, uid, &list_b1).await;
        let item_b1s = create_test_item(pool, &sublist_b1s).await;

        (
            root,
            SimpleTree {
                child,
                list_a1,
                item_a1,
                list_b1,
                item_b1,
                sublist_b1s,
                item_b1s,
            },
        )
    }

    struct SimpleTree {
        child: String,
        list_a1: String,
        item_a1: String,
        list_b1: String,
        item_b1: String,
        sublist_b1s: String,
        item_b1s: String,
    }

    async fn completed(pool: &SqlitePool, uid: &str, item_id: &str, secs: i32) {
        make_entry(
            pool,
            uid,
            Some(item_id),
            Some("2026-01-01 10:00:00"),
            Some(secs),
        )
        .await;
    }

    #[tokio::test]
    async fn item_total_sums_completed_plus_running_elapsed() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;

        completed(&pool, &uid, &item_id, 60).await;
        completed(&pool, &uid, &item_id, 120).await;

        let total = item_total(&pool, &uid, &item_id).await.unwrap();
        assert_eq!(total.seconds, 180);
        assert!(total.running_since.is_none());

        // A running entry adds its live elapsed time and is reported separately.
        insert(
            &pool,
            InsertTimeEntryInput {
                id: &Uuid::new_v4().to_string(),
                item_id: Some(&item_id),
                user_id: &uid,
                description: None,
                started_at: "2020-01-01 00:00:00", // long ago: elapsed is large
                source: "timer",
                mode: Some("stopwatch"),
                ended_at: None,
                duration: None,
            },
        )
        .await
        .unwrap();

        let total = item_total(&pool, &uid, &item_id).await.unwrap();
        assert!(total.seconds > 180, "running entry must be included");
        assert_eq!(total.running_since.as_deref(), Some("2020-01-01 00:00:00"));
    }

    #[tokio::test]
    async fn item_total_ignores_other_items_and_users() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;
        let other_item = create_test_item(&pool, &list_id).await;

        completed(&pool, &uid, &item_id, 60).await;
        completed(&pool, &uid, &other_item, 999).await;
        completed(&pool, &other, &item_id, 999).await;

        let total = item_total(&pool, &uid, &item_id).await.unwrap();
        assert_eq!(total.seconds, 60);
    }

    #[tokio::test]
    async fn inbox_entry_counts_nowhere() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let (root, tree) = nested_tree(&pool, &uid).await;
        let root_list = create_test_list(&pool, &uid).await;

        // Inbox entry: item_id NULL.
        make_entry(&pool, &uid, None, Some("2026-01-01 10:00:00"), Some(500)).await;

        assert_eq!(
            item_total(&pool, &uid, &tree.item_a1)
                .await
                .unwrap()
                .seconds,
            0
        );
        assert_eq!(
            list_total(&pool, &uid, &root_list).await.unwrap().seconds,
            0
        );
        assert_eq!(
            container_total(&pool, &uid, &root).await.unwrap().seconds,
            0
        );
    }

    #[tokio::test]
    async fn list_total_rolls_up_sublists_recursively() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let (_root, tree) = nested_tree(&pool, &uid).await;

        completed(&pool, &uid, &tree.item_b1, 100).await;
        completed(&pool, &uid, &tree.item_b1s, 20).await;

        // Parent list includes its sublist.
        let total = list_total(&pool, &uid, &tree.list_b1).await.unwrap();
        assert_eq!(total.seconds, 120);

        // The sublist only has its own items.
        let sub = list_total(&pool, &uid, &tree.sublist_b1s).await.unwrap();
        assert_eq!(sub.seconds, 20);
    }

    #[tokio::test]
    async fn container_total_rolls_up_subcontainers_and_sublists() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let (root, tree) = nested_tree(&pool, &uid).await;
        let outside_list = create_test_list(&pool, &uid).await;
        let outside_item = create_test_item(&pool, &outside_list).await;

        completed(&pool, &uid, &tree.item_a1, 10).await;
        completed(&pool, &uid, &tree.item_b1, 100).await;
        completed(&pool, &uid, &tree.item_b1s, 1000).await;
        completed(&pool, &uid, &outside_item, 5000).await;

        let total = container_total(&pool, &uid, &root).await.unwrap();
        assert_eq!(total.seconds, 1110);

        let child = container_total(&pool, &uid, &tree.child).await.unwrap();
        assert_eq!(child.seconds, 1100);
    }

    #[tokio::test]
    async fn totals_ignore_other_users_entries() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let (root, tree) = nested_tree(&pool, &uid).await;

        completed(&pool, &uid, &tree.item_b1, 100).await;
        completed(&pool, &other, &tree.item_b1, 700).await;

        assert_eq!(
            list_total(&pool, &uid, &tree.list_b1)
                .await
                .unwrap()
                .seconds,
            100
        );
        assert_eq!(
            container_total(&pool, &uid, &root).await.unwrap().seconds,
            100
        );
    }

    #[tokio::test]
    async fn item_totals_batches_in_one_query() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let (_root, tree) = nested_tree(&pool, &uid).await;

        completed(&pool, &uid, &tree.item_a1, 10).await;
        completed(&pool, &uid, &tree.item_b1, 100).await;

        let map = item_totals(
            &pool,
            &uid,
            &[
                tree.item_a1.clone(),
                tree.item_b1.clone(),
                tree.item_b1s.clone(),
            ],
        )
        .await
        .unwrap();
        assert_eq!(map.get(&tree.item_a1).unwrap().seconds, 10);
        assert_eq!(map.get(&tree.item_b1).unwrap().seconds, 100);
        // No entries: absent from the map.
        assert!(!map.contains_key(&tree.item_b1s));

        // Empty input short-circuits.
        assert!(item_totals(&pool, &uid, &[]).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn list_totals_roll_up_per_list() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let (_root, tree) = nested_tree(&pool, &uid).await;
        let empty_list = create_test_list(&pool, &uid).await;

        completed(&pool, &uid, &tree.item_a1, 10).await;
        completed(&pool, &uid, &tree.item_b1, 100).await;
        completed(&pool, &uid, &tree.item_b1s, 5).await;

        let map = list_totals(
            &pool,
            &uid,
            &[
                tree.list_a1.clone(),
                tree.list_b1.clone(),
                tree.sublist_b1s.clone(),
                empty_list.clone(),
            ],
        )
        .await
        .unwrap();
        assert_eq!(map.get(&tree.list_a1).unwrap().seconds, 10);
        // Parent list rolls its sublist up.
        assert_eq!(map.get(&tree.list_b1).unwrap().seconds, 105);
        assert_eq!(map.get(&tree.sublist_b1s).unwrap().seconds, 5);
        assert!(!map.contains_key(&empty_list));
    }

    #[tokio::test]
    async fn totals_for_unknown_entity_are_zero() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let missing = Uuid::new_v4().to_string();

        assert_eq!(item_total(&pool, &uid, &missing).await.unwrap().seconds, 0);
        assert_eq!(list_total(&pool, &uid, &missing).await.unwrap().seconds, 0);
        assert_eq!(
            container_total(&pool, &uid, &missing)
                .await
                .unwrap()
                .seconds,
            0
        );
    }

    #[tokio::test]
    async fn mode_is_stored_on_insert() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let entry = insert(
            &pool,
            InsertTimeEntryInput {
                id: &Uuid::new_v4().to_string(),
                item_id: None,
                user_id: &uid,
                description: None,
                started_at: "2026-01-01 10:00:00",
                source: "timer",
                mode: Some("pomodoro"),
                ended_at: None,
                duration: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(entry.mode.as_deref(), Some("pomodoro"));
    }
}
