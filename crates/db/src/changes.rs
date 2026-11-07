//! `change_log` reads and the commit-time wake-up signal (issue #356).
//!
//! Triggers in migration 019 record every write to user-visible data as a
//! `change_log` row, whatever the write path (server fns, REST, MCP batch
//! transactions, jobs). The server-side pump reads those rows with
//! [`fetch_after`] and waits on a [`ChangeSignal`], which
//! [`crate::create_pool_with_signal`] notifies from a SQLite commit hook.

use crate::{DbError, SqlitePool};
use sqlx::sqlite::{SqliteConnection, UpdateHookResult};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// One recorded change: the SSE event id and the topic the client has to reload.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChangeRow {
    pub id: i64,
    pub user_id: String,
    pub topic: String,
}

/// Rows with id > after_id across all users, oldest first, at most `limit`.
#[tracing::instrument(skip(pool))]
pub async fn fetch_after(
    pool: &SqlitePool,
    after_id: i64,
    limit: i64,
) -> Result<Vec<ChangeRow>, DbError> {
    sqlx::query_as::<_, ChangeRow>(
        "SELECT id, user_id, topic FROM change_log WHERE id > ? ORDER BY id ASC LIMIT ?",
    )
    .bind(after_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Rows of one user with id > after_id, oldest first, at most `limit` (Last-Event-ID replay).
#[tracing::instrument(skip(pool))]
pub async fn fetch_for_user_after(
    pool: &SqlitePool,
    user_id: &str,
    after_id: i64,
    limit: i64,
) -> Result<Vec<ChangeRow>, DbError> {
    sqlx::query_as::<_, ChangeRow>(
        "SELECT id, user_id, topic FROM change_log \
         WHERE user_id = ? AND id > ? ORDER BY id ASC LIMIT ?",
    )
    .bind(user_id)
    .bind(after_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// MAX(id) or 0 when empty.
#[tracing::instrument(skip(pool))]
pub async fn max_id(pool: &SqlitePool) -> Result<i64, DbError> {
    let id: (i64,) = sqlx::query_as("SELECT COALESCE(MAX(id), 0) FROM change_log")
        .fetch_one(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(id.0)
}

/// MIN(id) or None when empty (used to detect a replay gap after pruning).
#[tracing::instrument(skip(pool))]
pub async fn min_id(pool: &SqlitePool) -> Result<Option<i64>, DbError> {
    let id: (Option<i64>,) = sqlx::query_as("SELECT MIN(id) FROM change_log")
        .fetch_one(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(id.0)
}

/// Delete rows older than `older_than` (e.g. `chrono::Duration::hours(24)`); returns rows deleted.
#[tracing::instrument(skip(pool))]
pub async fn prune(pool: &SqlitePool, older_than: chrono::Duration) -> Result<u64, DbError> {
    // `at` is `datetime('now')` output (UTC, "YYYY-MM-DD HH:MM:SS"), which
    // compares correctly as text; the modifier does the date arithmetic in SQLite.
    let modifier = format!("-{} seconds", older_than.num_seconds().max(0));
    let result = sqlx::query("DELETE FROM change_log WHERE at < datetime('now', ?)")
        .bind(modifier)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(result.rows_affected())
}

/// Notified (notify_waiters + notify_one) after a COMMIT of a transaction that inserted into change_log.
#[derive(Clone, Default)]
pub struct ChangeSignal(pub Arc<tokio::sync::Notify>);

impl ChangeSignal {
    /// Wake every current waiter and leave a permit for the next one.
    pub fn notify(&self) {
        self.0.notify_waiters();
        self.0.notify_one();
    }
}

/// Register the update/commit/rollback hooks on one connection (see
/// [`crate::create_pool_with_signal`]).
///
/// The hooks run inside SQLite callbacks: they only touch an `AtomicBool` and the
/// signal, never the connection, and never block.
pub(crate) async fn install_hooks(
    conn: &mut SqliteConnection,
    signal: ChangeSignal,
) -> Result<(), sqlx::Error> {
    // One flag per connection: set while a transaction writes change_log, cleared
    // on commit or rollback. A commit hook that returns false would roll back, so
    // it always returns true.
    let wrote_change_log = Arc::new(AtomicBool::new(false));
    let mut handle = conn.lock_handle().await?;

    let flag = Arc::clone(&wrote_change_log);
    handle.set_update_hook(move |res: UpdateHookResult<'_>| {
        // `res.table` is only valid during this callback; compare it right away.
        if res.table == "change_log" {
            flag.store(true, Ordering::SeqCst);
        }
    });

    let flag = Arc::clone(&wrote_change_log);
    handle.set_commit_hook(move || {
        if flag.swap(false, Ordering::SeqCst) {
            signal.notify();
        }
        true
    });

    let flag = Arc::clone(&wrote_change_log);
    handle.set_rollback_hook(move || {
        flag.store(false, Ordering::SeqCst);
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{
        create_test_item, create_test_list, create_test_tag, create_test_user, test_pool,
    };
    use std::time::Duration;
    use uuid::Uuid;

    /// A seeded country (`PL`) for location rows.
    const COUNTRY_PL: &str = "01000000-0000-4000-a000-000000000178";

    fn new_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// `(user_id, topic, entity_id)` of every change_log row, oldest first.
    async fn log(pool: &SqlitePool) -> Vec<(String, String, Option<String>)> {
        sqlx::query_as("SELECT user_id, topic, entity_id FROM change_log ORDER BY id")
            .fetch_all(pool)
            .await
            .expect("read change_log")
    }

    /// Topics of every change_log row, oldest first.
    async fn topics(pool: &SqlitePool) -> Vec<String> {
        log(pool)
            .await
            .into_iter()
            .map(|(_, topic, _)| topic)
            .collect()
    }

    async fn clear_log(pool: &SqlitePool) {
        sqlx::query("DELETE FROM change_log")
            .execute(pool)
            .await
            .expect("clear change_log");
    }

    async fn exec(pool: &SqlitePool, sql: &str, id: &str) {
        sqlx::query(sql)
            .bind(id)
            .execute(pool)
            .await
            .unwrap_or_else(|e| panic!("{sql}: {e}"));
    }

    async fn insert_container(pool: &SqlitePool, user_id: &str) -> String {
        let id = new_id();
        sqlx::query("INSERT INTO containers (id, user_id, name) VALUES (?, ?, 'Box')")
            .bind(&id)
            .bind(user_id)
            .execute(pool)
            .await
            .expect("insert_container");
        id
    }

    async fn insert_template(pool: &SqlitePool, user_id: &str) -> String {
        let id = new_id();
        sqlx::query("INSERT INTO templates (id, user_id, name) VALUES (?, ?, 'Template')")
            .bind(&id)
            .bind(user_id)
            .execute(pool)
            .await
            .expect("insert_template");
        id
    }

    async fn insert_template_item(pool: &SqlitePool, template_id: &str) -> String {
        let id = new_id();
        sqlx::query("INSERT INTO template_items (id, template_id, title) VALUES (?, ?, 'Line')")
            .bind(&id)
            .bind(template_id)
            .execute(pool)
            .await
            .expect("insert_template_item");
        id
    }

    async fn insert_comment(pool: &SqlitePool, user_id: &str) -> String {
        let id = new_id();
        sqlx::query(
            "INSERT INTO comments (id, entity_type, entity_id, content, user_id) \
             VALUES (?, 'item', 'gone', 'hello', ?)",
        )
        .bind(&id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("insert_comment");
        id
    }

    async fn insert_relation(pool: &SqlitePool, user_id: &str) -> String {
        let id = new_id();
        sqlx::query(
            "INSERT INTO entity_relations (id, from_type, from_id, to_type, to_id, relation_type, user_id) \
             VALUES (?, 'item', 'a', 'item', 'b', 'related', ?)",
        )
        .bind(&id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("insert_relation");
        id
    }

    async fn insert_time_entry(pool: &SqlitePool, user_id: &str) -> String {
        let id = new_id();
        sqlx::query(
            "INSERT INTO time_entries (id, user_id, started_at, source) \
             VALUES (?, ?, '2026-01-01T10:00:00Z', 'manual')",
        )
        .bind(&id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("insert_time_entry");
        id
    }

    async fn insert_location(pool: &SqlitePool, user_id: &str) -> String {
        let id = new_id();
        sqlx::query(
            "INSERT INTO locations (id, user_id, name, type, country_id) VALUES (?, ?, 'Kraków', 'city', ?)",
        )
        .bind(&id)
        .bind(user_id)
        .bind(COUNTRY_PL)
        .execute(pool)
        .await
        .expect("insert_location");
        id
    }

    async fn insert_calendar_token(pool: &SqlitePool, user_id: &str) -> String {
        let id = new_id();
        sqlx::query(
            "INSERT INTO calendar_tokens (id, user_id, name, token_hash) VALUES (?, ?, 'Feed', ?)",
        )
        .bind(&id)
        .bind(user_id)
        .bind(new_id())
        .execute(pool)
        .await
        .expect("insert_calendar_token");
        id
    }

    async fn insert_personal_token(pool: &SqlitePool, user_id: &str) -> String {
        let id = new_id();
        sqlx::query("INSERT INTO personal_tokens (id, user_id, name) VALUES (?, ?, 'CLI')")
            .bind(&id)
            .bind(user_id)
            .execute(pool)
            .await
            .expect("insert_personal_token");
        id
    }

    async fn link_item_tag(pool: &SqlitePool, item_id: &str, tag_id: &str) {
        sqlx::query("INSERT INTO item_tags (item_id, tag_id) VALUES (?, ?)")
            .bind(item_id)
            .bind(tag_id)
            .execute(pool)
            .await
            .expect("link_item_tag");
    }

    async fn link_list_tag(pool: &SqlitePool, list_id: &str, tag_id: &str) {
        sqlx::query("INSERT INTO list_tags (list_id, tag_id) VALUES (?, ?)")
            .bind(list_id)
            .bind(tag_id)
            .execute(pool)
            .await
            .expect("link_list_tag");
    }

    async fn link_container_tag(pool: &SqlitePool, container_id: &str, tag_id: &str) {
        sqlx::query("INSERT INTO container_tags (container_id, tag_id) VALUES (?, ?)")
            .bind(container_id)
            .bind(tag_id)
            .execute(pool)
            .await
            .expect("link_container_tag");
    }

    async fn link_template_tag(pool: &SqlitePool, template_id: &str, tag_id: &str) {
        sqlx::query("INSERT INTO template_tags (template_id, tag_id) VALUES (?, ?)")
            .bind(template_id)
            .bind(tag_id)
            .execute(pool)
            .await
            .expect("link_template_tag");
    }

    /// Pool with the change hooks installed, migrations applied, and its signal.
    async fn signal_pool() -> (SqlitePool, ChangeSignal) {
        let signal = ChangeSignal::default();
        let pool = crate::create_pool_with_signal(":memory:", signal.clone())
            .await
            .expect("signal pool creation");
        crate::run_migrations(&pool).await.expect("test migrations");
        (pool, signal)
    }

    async fn notified_within(signal: &ChangeSignal, window: Duration) -> bool {
        tokio::time::timeout(window, signal.0.notified())
            .await
            .is_ok()
    }

    // ── one row per mapped table ─────────────────────────────────────────────

    #[tokio::test]
    async fn lists_insert_update_delete_are_logged() {
        let pool = test_pool().await;
        let user = create_test_user(&pool).await;
        clear_log(&pool).await;

        let list = create_test_list(&pool, &user).await;
        assert_eq!(
            log(&pool).await,
            vec![(user.clone(), "lists".to_string(), Some(list.clone()))]
        );

        clear_log(&pool).await;
        exec(
            &pool,
            "UPDATE lists SET name = 'Renamed' WHERE id = ?",
            &list,
        )
        .await;
        assert_eq!(
            log(&pool).await,
            vec![(user.clone(), "lists".into(), Some(list.clone()))]
        );

        clear_log(&pool).await;
        exec(&pool, "DELETE FROM lists WHERE id = ?", &list).await;
        assert_eq!(log(&pool).await, vec![(user, "lists".into(), Some(list))]);
    }

    #[tokio::test]
    async fn containers_insert_update_delete_are_logged() {
        let pool = test_pool().await;
        let user = create_test_user(&pool).await;
        clear_log(&pool).await;

        let container = insert_container(&pool, &user).await;
        assert_eq!(
            log(&pool).await,
            vec![(
                user.clone(),
                "containers".to_string(),
                Some(container.clone())
            )]
        );

        clear_log(&pool).await;
        exec(
            &pool,
            "UPDATE containers SET archived = 1 WHERE id = ?",
            &container,
        )
        .await;
        assert_eq!(topics(&pool).await, vec!["containers"]);

        clear_log(&pool).await;
        exec(&pool, "DELETE FROM containers WHERE id = ?", &container).await;
        assert_eq!(
            log(&pool).await,
            vec![(user, "containers".into(), Some(container))]
        );
    }

    /// Opening a list or container writes last_opened_at only: that is a read of
    /// the user's data and must not wake the live-update pump. Any other UPDATE
    /// (last_opened_at untouched) fires, including columns added later.
    #[tokio::test]
    async fn last_opened_at_touches_are_not_logged() {
        let pool = test_pool().await;
        let user = create_test_user(&pool).await;
        let list = create_test_list(&pool, &user).await;
        let container = insert_container(&pool, &user).await;
        clear_log(&pool).await;

        // The real touch functions, twice each: the second call in the same
        // second must write nothing (it would otherwise look like a real change).
        for _ in 0..2 {
            crate::lists::touch_last_opened(&pool, &list, &user)
                .await
                .unwrap();
            crate::containers::touch_last_opened(&pool, &container, &user)
                .await
                .unwrap();
        }
        assert!(log(&pool).await.is_empty(), "touch writes must stay silent");

        // Updates that leave last_opened_at alone fire.
        exec(&pool, "UPDATE lists SET pinned = 1 WHERE id = ?", &list).await;
        exec(
            &pool,
            "UPDATE containers SET name = 'Renamed' WHERE id = ?",
            &container,
        )
        .await;
        assert_eq!(topics(&pool).await, vec!["lists", "containers"]);
    }

    #[tokio::test]
    async fn items_are_logged_under_the_list_owner() {
        let pool = test_pool().await;
        let user = create_test_user(&pool).await;
        let list = create_test_list(&pool, &user).await;
        clear_log(&pool).await;

        let item = create_test_item(&pool, &list).await;
        assert_eq!(
            log(&pool).await,
            vec![(user.clone(), "items".to_string(), Some(item.clone()))]
        );

        clear_log(&pool).await;
        exec(
            &pool,
            "UPDATE items SET title = 'Renamed' WHERE id = ?",
            &item,
        )
        .await;
        assert_eq!(
            log(&pool).await,
            vec![(user.clone(), "items".into(), Some(item.clone()))]
        );

        clear_log(&pool).await;
        exec(&pool, "DELETE FROM items WHERE id = ?", &item).await;
        assert_eq!(log(&pool).await, vec![(user, "items".into(), Some(item))]);
    }

    #[tokio::test]
    async fn tags_insert_update_delete_are_logged() {
        let pool = test_pool().await;
        let user = create_test_user(&pool).await;
        clear_log(&pool).await;

        let tag = create_test_tag(&pool, &user, "camping").await;
        assert_eq!(
            log(&pool).await,
            vec![(user.clone(), "tags".to_string(), Some(tag.clone()))]
        );

        clear_log(&pool).await;
        exec(&pool, "UPDATE tags SET name = 'renamed' WHERE id = ?", &tag).await;
        assert_eq!(
            log(&pool).await,
            vec![(user.clone(), "tags".into(), Some(tag.clone()))]
        );

        clear_log(&pool).await;
        exec(&pool, "DELETE FROM tags WHERE id = ?", &tag).await;
        assert_eq!(log(&pool).await, vec![(user, "tags".into(), Some(tag))]);
    }

    #[tokio::test]
    async fn item_tags_produce_items_and_tags_rows() {
        let pool = test_pool().await;
        let user = create_test_user(&pool).await;
        let list = create_test_list(&pool, &user).await;
        let item = create_test_item(&pool, &list).await;
        let tag = create_test_tag(&pool, &user, "camping").await;
        clear_log(&pool).await;

        link_item_tag(&pool, &item, &tag).await;
        let mut got = topics(&pool).await;
        got.sort();
        assert_eq!(got, vec!["items", "tags"]);
        assert!(
            log(&pool)
                .await
                .iter()
                .all(|(u, _, e)| u == &user && e.is_none())
        );

        clear_log(&pool).await;
        sqlx::query("DELETE FROM item_tags WHERE item_id = ?")
            .bind(&item)
            .execute(&pool)
            .await
            .unwrap();
        let mut got = topics(&pool).await;
        got.sort();
        assert_eq!(got, vec!["items", "tags"]);
    }

    #[tokio::test]
    async fn list_tags_produce_lists_and_tags_rows() {
        let pool = test_pool().await;
        let user = create_test_user(&pool).await;
        let list = create_test_list(&pool, &user).await;
        let tag = create_test_tag(&pool, &user, "camping").await;
        clear_log(&pool).await;

        link_list_tag(&pool, &list, &tag).await;
        let mut got = topics(&pool).await;
        got.sort();
        assert_eq!(got, vec!["lists", "tags"]);
        assert!(log(&pool).await.iter().all(|(u, _, _)| u == &user));

        clear_log(&pool).await;
        sqlx::query("DELETE FROM list_tags WHERE list_id = ?")
            .bind(&list)
            .execute(&pool)
            .await
            .unwrap();
        let mut got = topics(&pool).await;
        got.sort();
        assert_eq!(got, vec!["lists", "tags"]);
    }

    #[tokio::test]
    async fn container_tags_produce_containers_and_tags_rows() {
        let pool = test_pool().await;
        let user = create_test_user(&pool).await;
        let container = insert_container(&pool, &user).await;
        let tag = create_test_tag(&pool, &user, "camping").await;
        clear_log(&pool).await;

        link_container_tag(&pool, &container, &tag).await;
        let mut got = topics(&pool).await;
        got.sort();
        assert_eq!(got, vec!["containers", "tags"]);
        assert!(log(&pool).await.iter().all(|(u, _, _)| u == &user));

        clear_log(&pool).await;
        sqlx::query("DELETE FROM container_tags WHERE container_id = ?")
            .bind(&container)
            .execute(&pool)
            .await
            .unwrap();
        let mut got = topics(&pool).await;
        got.sort();
        assert_eq!(got, vec!["containers", "tags"]);
    }

    #[tokio::test]
    async fn templates_and_template_items_are_logged_as_templates() {
        let pool = test_pool().await;
        let user = create_test_user(&pool).await;
        clear_log(&pool).await;

        let template = insert_template(&pool, &user).await;
        assert_eq!(
            log(&pool).await,
            vec![(
                user.clone(),
                "templates".to_string(),
                Some(template.clone())
            )]
        );

        clear_log(&pool).await;
        let line = insert_template_item(&pool, &template).await;
        assert_eq!(
            log(&pool).await,
            vec![(user.clone(), "templates".to_string(), Some(line.clone()))],
            "template_items resolve the owner through the template"
        );

        clear_log(&pool).await;
        exec(
            &pool,
            "UPDATE template_items SET title = 'Renamed' WHERE id = ?",
            &line,
        )
        .await;
        assert_eq!(topics(&pool).await, vec!["templates"]);

        clear_log(&pool).await;
        exec(&pool, "DELETE FROM template_items WHERE id = ?", &line).await;
        assert_eq!(
            log(&pool).await,
            vec![(user.clone(), "templates".into(), Some(line))]
        );

        clear_log(&pool).await;
        exec(
            &pool,
            "UPDATE templates SET name = 'Renamed' WHERE id = ?",
            &template,
        )
        .await;
        assert_eq!(topics(&pool).await, vec!["templates"]);

        clear_log(&pool).await;
        exec(&pool, "DELETE FROM templates WHERE id = ?", &template).await;
        assert_eq!(
            log(&pool).await,
            vec![(user, "templates".into(), Some(template))]
        );
    }

    #[tokio::test]
    async fn template_tags_produce_a_templates_row() {
        let pool = test_pool().await;
        let user = create_test_user(&pool).await;
        let template = insert_template(&pool, &user).await;
        let tag = create_test_tag(&pool, &user, "camping").await;
        clear_log(&pool).await;

        link_template_tag(&pool, &template, &tag).await;
        assert_eq!(
            log(&pool).await,
            vec![(user.clone(), "templates".into(), None)]
        );

        clear_log(&pool).await;
        sqlx::query("DELETE FROM template_tags WHERE template_id = ?")
            .bind(&template)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(topics(&pool).await, vec!["templates"]);
    }

    #[tokio::test]
    async fn comments_relations_and_time_entries_are_logged() {
        let pool = test_pool().await;
        let user = create_test_user(&pool).await;
        clear_log(&pool).await;

        let comment = insert_comment(&pool, &user).await;
        assert_eq!(topics(&pool).await, vec!["comments"]);
        clear_log(&pool).await;
        exec(
            &pool,
            "UPDATE comments SET content = 'edited' WHERE id = ?",
            &comment,
        )
        .await;
        assert_eq!(
            log(&pool).await,
            vec![(user.clone(), "comments".into(), Some(comment.clone()))]
        );
        clear_log(&pool).await;
        exec(&pool, "DELETE FROM comments WHERE id = ?", &comment).await;
        assert_eq!(topics(&pool).await, vec!["comments"]);

        clear_log(&pool).await;
        let relation = insert_relation(&pool, &user).await;
        assert_eq!(topics(&pool).await, vec!["relations"]);
        clear_log(&pool).await;
        exec(
            &pool,
            "UPDATE entity_relations SET relation_type = 'blocks' WHERE id = ?",
            &relation,
        )
        .await;
        assert_eq!(topics(&pool).await, vec!["relations"]);
        clear_log(&pool).await;
        exec(
            &pool,
            "DELETE FROM entity_relations WHERE id = ?",
            &relation,
        )
        .await;
        assert_eq!(topics(&pool).await, vec!["relations"]);

        clear_log(&pool).await;
        let entry = insert_time_entry(&pool, &user).await;
        assert_eq!(
            log(&pool).await,
            vec![(user.clone(), "time_entries".into(), Some(entry.clone()))]
        );
        clear_log(&pool).await;
        exec(
            &pool,
            "UPDATE time_entries SET description = 'x' WHERE id = ?",
            &entry,
        )
        .await;
        assert_eq!(topics(&pool).await, vec!["time_entries"]);
        clear_log(&pool).await;
        exec(&pool, "DELETE FROM time_entries WHERE id = ?", &entry).await;
        assert_eq!(topics(&pool).await, vec!["time_entries"]);
    }

    #[tokio::test]
    async fn locations_insert_update_delete_are_logged() {
        let pool = test_pool().await;
        let user = create_test_user(&pool).await;
        clear_log(&pool).await;

        let location = insert_location(&pool, &user).await;
        assert_eq!(
            log(&pool).await,
            vec![(user.clone(), "locations".into(), Some(location.clone()))]
        );

        clear_log(&pool).await;
        exec(
            &pool,
            "UPDATE locations SET alias = 'home' WHERE id = ?",
            &location,
        )
        .await;
        assert_eq!(topics(&pool).await, vec!["locations"]);

        clear_log(&pool).await;
        exec(&pool, "DELETE FROM locations WHERE id = ?", &location).await;
        assert_eq!(topics(&pool).await, vec!["locations"]);
    }

    #[tokio::test]
    async fn settings_tables_are_logged_as_settings() {
        let pool = test_pool().await;
        let user = create_test_user(&pool).await;
        clear_log(&pool).await;

        sqlx::query("INSERT INTO user_settings (user_id, key, value) VALUES (?, 'k', 'v')")
            .bind(&user)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            log(&pool).await,
            vec![(user.clone(), "settings".into(), None)],
            "composite key"
        );
        clear_log(&pool).await;
        sqlx::query("UPDATE user_settings SET value = 'w' WHERE user_id = ?")
            .bind(&user)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(topics(&pool).await, vec!["settings"]);
        clear_log(&pool).await;
        sqlx::query("DELETE FROM user_settings WHERE user_id = ?")
            .bind(&user)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(topics(&pool).await, vec!["settings"]);

        clear_log(&pool).await;
        let calendar_token = insert_calendar_token(&pool, &user).await;
        assert_eq!(
            log(&pool).await,
            vec![(
                user.clone(),
                "settings".into(),
                Some(calendar_token.clone())
            )]
        );
        clear_log(&pool).await;
        exec(
            &pool,
            "DELETE FROM calendar_tokens WHERE id = ?",
            &calendar_token,
        )
        .await;
        assert_eq!(topics(&pool).await, vec!["settings"]);

        clear_log(&pool).await;
        let personal_token = insert_personal_token(&pool, &user).await;
        assert_eq!(
            log(&pool).await,
            vec![(
                user.clone(),
                "settings".into(),
                Some(personal_token.clone())
            )]
        );
        clear_log(&pool).await;
        exec(
            &pool,
            "UPDATE personal_tokens SET last_used_at = datetime('now') WHERE id = ?",
            &personal_token,
        )
        .await;
        assert_eq!(topics(&pool).await, vec!["settings"]);
        clear_log(&pool).await;
        exec(
            &pool,
            "DELETE FROM personal_tokens WHERE id = ?",
            &personal_token,
        )
        .await;
        assert_eq!(topics(&pool).await, vec!["settings"]);
    }

    /// Cascade deletes fire the child triggers after the parent row is gone, so
    /// their owner resolves to NULL; those rows are skipped, not inserted, and the
    /// parent event still reaches the owner.
    #[tokio::test]
    async fn deleting_a_list_with_items_and_tags_succeeds() {
        let pool = test_pool().await;
        let user = create_test_user(&pool).await;
        let list = create_test_list(&pool, &user).await;
        let item = create_test_item(&pool, &list).await;
        let tag = create_test_tag(&pool, &user, "camping").await;
        link_item_tag(&pool, &item, &tag).await;
        link_list_tag(&pool, &list, &tag).await;
        clear_log(&pool).await;

        exec(&pool, "DELETE FROM lists WHERE id = ?", &list).await;

        let rows = log(&pool).await;
        assert!(
            rows.iter().any(|(u, t, _)| u == &user && t == "lists"),
            "expected a lists row for the owner, got {rows:?}"
        );
        assert!(
            rows.iter().all(|(u, _, _)| u == &user),
            "every row must carry the owner, got {rows:?}"
        );
    }

    // ── reads ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn fetch_after_and_fetch_for_user_after() {
        let pool = test_pool().await;
        let user_a = create_test_user(&pool).await;
        let user_b = create_test_user(&pool).await;
        clear_log(&pool).await;
        for _ in 0..3 {
            create_test_list(&pool, &user_a).await;
        }
        create_test_list(&pool, &user_b).await;

        let all = fetch_after(&pool, 0, 100).await.unwrap();
        assert_eq!(all.len(), 4);
        assert!(all.windows(2).all(|w| w[0].id < w[1].id), "oldest first");
        assert_eq!(all.iter().filter(|r| r.user_id == user_a).count(), 3);
        assert!(all.iter().all(|r| r.topic == "lists"));

        let limited = fetch_after(&pool, 0, 2).await.unwrap();
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].id, all[0].id);

        let after = fetch_after(&pool, all[1].id, 100).await.unwrap();
        assert_eq!(after.len(), 2);
        assert_eq!(after[0].id, all[2].id);

        let mine = fetch_for_user_after(&pool, &user_b, 0, 100).await.unwrap();
        assert_eq!(mine.len(), 1);
        assert_eq!(mine[0].user_id, user_b);
        assert_eq!(mine[0].id, all.last().unwrap().id);

        assert!(
            fetch_for_user_after(&pool, &user_b, mine[0].id, 100)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn max_id_and_min_id_track_the_log() {
        let pool = test_pool().await;
        let user = create_test_user(&pool).await;
        clear_log(&pool).await;

        assert_eq!(max_id(&pool).await.unwrap(), 0);
        assert_eq!(min_id(&pool).await.unwrap(), None);

        create_test_list(&pool, &user).await;
        create_test_list(&pool, &user).await;

        let rows = fetch_after(&pool, 0, 10).await.unwrap();
        assert_eq!(max_id(&pool).await.unwrap(), rows.last().unwrap().id);
        assert_eq!(min_id(&pool).await.unwrap(), Some(rows[0].id));
        assert!(rows[0].id < rows[1].id);
    }

    #[tokio::test]
    async fn prune_deletes_only_rows_older_than_the_window() {
        let pool = test_pool().await;
        let user = create_test_user(&pool).await;
        clear_log(&pool).await;
        create_test_list(&pool, &user).await;
        create_test_list(&pool, &user).await;

        let old_id = min_id(&pool).await.unwrap().unwrap();
        sqlx::query("UPDATE change_log SET at = datetime('now', '-48 hours') WHERE id = ?")
            .bind(old_id)
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(prune(&pool, chrono::Duration::hours(24)).await.unwrap(), 1);
        let left = fetch_after(&pool, 0, 10).await.unwrap();
        assert_eq!(left.len(), 1);
        assert!(left[0].id > old_id);
        assert_eq!(prune(&pool, chrono::Duration::hours(24)).await.unwrap(), 0);
    }

    // ── signal ───────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn change_signal_notifies_on_a_change_log_commit() {
        let (pool, signal) = signal_pool().await;
        let user = create_test_user(&pool).await;

        create_test_list(&pool, &user).await;
        assert!(
            notified_within(&signal, Duration::from_secs(1)).await,
            "a committed change_log write must notify the signal"
        );

        // The flag is cleared by the commit, so the next change is a new wake-up.
        create_test_list(&pool, &user).await;
        assert!(
            notified_within(&signal, Duration::from_secs(1)).await,
            "a second commit must notify again"
        );
    }

    #[tokio::test]
    async fn change_signal_ignores_reads_and_unrelated_writes() {
        let (pool, signal) = signal_pool().await;
        let user = create_test_user(&pool).await;

        let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM users")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, user);
        sqlx::query("INSERT INTO server_config (key, value) VALUES ('theme', 'dark')")
            .execute(&pool)
            .await
            .unwrap();

        assert!(
            !notified_within(&signal, Duration::from_millis(300)).await,
            "a read and a write to an unrelated table must not notify"
        );
    }

    #[tokio::test]
    async fn change_signal_ignores_a_rolled_back_write() {
        let (pool, signal) = signal_pool().await;
        let user = create_test_user(&pool).await;

        let mut tx = pool.begin().await.unwrap();
        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES (?, ?, 'Gone')")
            .bind(new_id())
            .bind(&user)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.rollback().await.unwrap();

        assert!(
            !notified_within(&signal, Duration::from_millis(300)).await,
            "a rolled back write must not notify"
        );
    }
}
