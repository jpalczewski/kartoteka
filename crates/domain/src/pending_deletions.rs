//! The pending-deletions queue behind `/trash` (#226).
//!
//! Marking an entity writes a row into `pending_deletions` instead of deleting it, so
//! the user decides later. MCP agents can only mark; nothing hard-deletes outside this
//! queue (plus the ordinary list/container/item/comment endpoints).
//!
//! Deleting from the queue cascades: a marked list or container goes with its whole
//! subtree **including items**. That is the only path that bypasses the #223 rule
//! "items always block". Every other path keeps calling `delete_tree(.., false)`.
//!
//! Rows whose entity disappears by any path are removed by the migration-021 triggers,
//! so the queue never points at a missing entity.

use crate::DomainError;
use kartoteka_db as db;
use kartoteka_db::types::DeleteTreeOutcome;
use kartoteka_shared::{DeletableEntity, PendingDeletionItem, PendingDeletionKey};
use sqlx::SqlitePool;
use std::collections::HashSet;

/// Longest label rendered on `/trash`; longer text is cut on a char boundary and ends
/// with `…`, so the result is at most `MAX_LABEL_CHARS` characters.
const MAX_LABEL_CHARS: usize = 120;

/// Cut `text` to [`MAX_LABEL_CHARS`] characters without splitting a UTF-8 sequence.
fn truncate_label(text: &str) -> String {
    if text.chars().count() <= MAX_LABEL_CHARS {
        return text.to_string();
    }
    let mut out: String = text.chars().take(MAX_LABEL_CHARS - 1).collect();
    out.push('…');
    out
}

/// Missing and foreign entities are indistinguishable: both are `NotFound(kind)`.
async fn ensure_owned(
    pool: &SqlitePool,
    user_id: &str,
    kind: DeletableEntity,
    entity_id: &str,
) -> Result<(), DomainError> {
    let exists = match kind {
        DeletableEntity::Item => db::items::get_one(pool, entity_id, user_id)
            .await?
            .is_some(),
        DeletableEntity::List => db::lists::get_one(pool, entity_id, user_id)
            .await?
            .is_some(),
        DeletableEntity::Container => db::containers::get_one(pool, entity_id, user_id)
            .await?
            .is_some(),
        DeletableEntity::Comment => db::comments::get_owned(pool, entity_id, user_id)
            .await?
            .is_some(),
    };
    if !exists {
        return Err(DomainError::NotFound(kind.as_str()));
    }
    Ok(())
}

/// Marks or unmarks. Returns true when the entity is now marked.
///
/// # Errors
/// `NotFound(kind)` for a missing or foreign entity.
#[tracing::instrument(skip(pool))]
pub async fn toggle(
    pool: &SqlitePool,
    user_id: &str,
    kind: DeletableEntity,
    entity_id: &str,
) -> Result<bool, DomainError> {
    ensure_owned(pool, user_id, kind, entity_id).await?;

    let kind_str = kind.as_str();
    if db::pending_deletions::is_marked(pool, kind_str, entity_id, user_id).await? {
        db::pending_deletions::remove(pool, kind_str, entity_id, user_id).await?;
        Ok(false)
    } else {
        db::pending_deletions::insert(pool, kind_str, entity_id, user_id).await?;
        Ok(true)
    }
}

/// The user's queue, resolved for display and ordered newest first.
///
/// Labels, context and links are resolved with point queries; a row whose entity has
/// just disappeared (race with a cascade) is skipped.
///
/// # Errors
/// Propagates database errors.
#[tracing::instrument(skip(pool))]
pub async fn list_all(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<PendingDeletionItem>, DomainError> {
    let rows = db::pending_deletions::list_for_user(pool, user_id).await?;
    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let Ok(kind) = row.entity_type.parse::<DeletableEntity>() else {
            continue;
        };
        if let Some(item) = build_item(pool, user_id, kind, &row.entity_id, &row.marked_at).await? {
            items.push(item);
        }
    }
    Ok(items)
}

async fn build_item(
    pool: &SqlitePool,
    user_id: &str,
    kind: DeletableEntity,
    entity_id: &str,
    marked_at: &str,
) -> Result<Option<PendingDeletionItem>, DomainError> {
    let resolved = match kind {
        DeletableEntity::Item => {
            let Some(item) = db::items::get_one(pool, entity_id, user_id).await? else {
                return Ok(None);
            };
            let list_name = db::lists::get_one(pool, &item.list_id, user_id)
                .await?
                .map(|list| list.name);
            (
                truncate_label(&item.title),
                list_name,
                Some(format!("/lists/{}/items/{}", item.list_id, item.id)),
                None,
            )
        }
        DeletableEntity::List => {
            let Some(list) = db::lists::get_one(pool, entity_id, user_id).await? else {
                return Ok(None);
            };
            // A list hangs either in a container or under another list.
            let context = if let Some(container_id) = list.container_id.as_deref() {
                db::containers::get_one(pool, container_id, user_id)
                    .await?
                    .map(|container| container.name)
            } else if let Some(parent_id) = list.parent_list_id.as_deref() {
                db::lists::get_one(pool, parent_id, user_id)
                    .await?
                    .map(|parent| parent.name)
            } else {
                None
            };
            let contains = db::lists::subtree_summary(pool, entity_id, user_id)
                .await?
                .map(crate::delete_preview);
            (
                truncate_label(&list.name),
                context,
                Some(format!("/lists/{entity_id}")),
                contains,
            )
        }
        DeletableEntity::Container => {
            let Some(container) = db::containers::get_one(pool, entity_id, user_id).await? else {
                return Ok(None);
            };
            let context = if let Some(parent_id) = container.parent_container_id.as_deref() {
                db::containers::get_one(pool, parent_id, user_id)
                    .await?
                    .map(|parent| parent.name)
            } else {
                None
            };
            let contains = db::containers::subtree_summary(pool, entity_id, user_id)
                .await?
                .map(crate::delete_preview);
            (
                truncate_label(&container.name),
                context,
                Some(format!("/containers/{entity_id}")),
                contains,
            )
        }
        DeletableEntity::Comment => {
            let Some(comment) = db::comments::get_owned(pool, entity_id, user_id).await? else {
                return Ok(None);
            };
            let (context, href) =
                comment_target(pool, user_id, &comment.entity_type, &comment.entity_id).await?;
            (truncate_label(&comment.content), context, href, None)
        }
    };

    let (label, context, href, contains) = resolved;
    Ok(Some(PendingDeletionItem {
        entity_type: kind,
        entity_id: entity_id.to_string(),
        label,
        context,
        href,
        marked_at: marked_at.to_string(),
        contains,
    }))
}

/// Where a comment lives and what it comments on: `(label of the entity, its link)`.
async fn comment_target(
    pool: &SqlitePool,
    user_id: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<(Option<String>, Option<String>), DomainError> {
    match entity_type {
        "item" => {
            let Some(item) = db::items::get_one(pool, entity_id, user_id).await? else {
                return Ok((None, None));
            };
            Ok((
                Some(truncate_label(&item.title)),
                Some(format!("/lists/{}/items/{}", item.list_id, item.id)),
            ))
        }
        "list" => {
            let Some(list) = db::lists::get_one(pool, entity_id, user_id).await? else {
                return Ok((None, None));
            };
            Ok((
                Some(truncate_label(&list.name)),
                Some(format!("/lists/{}", list.id)),
            ))
        }
        "container" => {
            let Some(container) = db::containers::get_one(pool, entity_id, user_id).await? else {
                return Ok((None, None));
            };
            Ok((
                Some(truncate_label(&container.name)),
                Some(format!("/containers/{}", container.id)),
            ))
        }
        _ => Ok((None, None)),
    }
}

/// Removes keys from the queue without deleting the entities. Returns the number
/// removed; keys that were not queued are ignored.
///
/// # Errors
/// Propagates database errors.
#[tracing::instrument(skip(pool))]
pub async fn unmark(
    pool: &SqlitePool,
    user_id: &str,
    keys: &[PendingDeletionKey],
) -> Result<u32, DomainError> {
    let mut removed = 0;
    for key in keys {
        if db::pending_deletions::remove(pool, key.entity_type.as_str(), &key.entity_id, user_id)
            .await?
        {
            removed += 1;
        }
    }
    Ok(removed)
}

/// Hard-deletes the given entities. Every key must be queued for this user, otherwise
/// `NotFound("pending_deletion")` comes back before anything is deleted.
///
/// Rows are processed containers first, then lists, items and comments, so a batch that
/// nests an item in a marked list deletes the list (taking the item along) and then
/// skips the item: whatever goes first takes the other along, and only entities that
/// were actually deleted are counted.
///
/// # Errors
/// `NotFound("pending_deletion")` for a key outside the queue, or database errors.
#[tracing::instrument(skip(pool))]
pub async fn delete(
    pool: &SqlitePool,
    user_id: &str,
    keys: &[PendingDeletionKey],
) -> Result<u32, DomainError> {
    if keys.is_empty() {
        return Ok(0);
    }

    let queued: HashSet<(String, String)> = db::pending_deletions::list_for_user(pool, user_id)
        .await?
        .into_iter()
        .map(|row| (row.entity_type, row.entity_id))
        .collect();
    for key in keys {
        if !queued.contains(&(key.entity_type.as_str().to_string(), key.entity_id.clone())) {
            return Err(DomainError::NotFound("pending_deletion"));
        }
    }

    let mut ordered = Vec::with_capacity(keys.len());
    for kind in [
        DeletableEntity::Container,
        DeletableEntity::List,
        DeletableEntity::Item,
        DeletableEntity::Comment,
    ] {
        ordered.extend(keys.iter().filter(|key| key.entity_type == kind));
    }

    let mut deleted = 0;
    for key in ordered {
        let gone = match key.entity_type {
            DeletableEntity::Container => {
                matches!(
                    db::containers::delete_tree(pool, &key.entity_id, user_id, true).await?,
                    DeleteTreeOutcome::Deleted
                )
            }
            DeletableEntity::List => {
                matches!(
                    db::lists::delete_tree(pool, &key.entity_id, user_id, true).await?,
                    DeleteTreeOutcome::Deleted
                )
            }
            DeletableEntity::Item => db::items::delete(pool, &key.entity_id, user_id).await?,
            DeletableEntity::Comment => db::comments::delete(pool, &key.entity_id, user_id).await?,
        };
        if gone {
            deleted += 1;
        }
    }
    Ok(deleted)
}

/// Hard-deletes the whole queue of the user. Entities already taken along by a cascade
/// are skipped rather than reported as an error.
///
/// # Errors
/// Propagates database errors.
#[tracing::instrument(skip(pool))]
pub async fn delete_all(pool: &SqlitePool, user_id: &str) -> Result<u32, DomainError> {
    let keys: Vec<PendingDeletionKey> = db::pending_deletions::list_for_user(pool, user_id)
        .await?
        .into_iter()
        .filter_map(|row| {
            row.entity_type
                .parse::<DeletableEntity>()
                .ok()
                .map(|entity_type| PendingDeletionKey {
                    entity_type,
                    entity_id: row.entity_id,
                })
        })
        .collect();
    delete(pool, user_id, &keys).await
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{create_test_user, test_pool};
    use uuid::Uuid;

    async fn make_list(
        pool: &SqlitePool,
        user_id: &str,
        name: &str,
        parent: Option<&str>,
        container: Option<&str>,
    ) -> String {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO lists (id, user_id, name, parent_list_id, container_id) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(user_id)
        .bind(name)
        .bind(parent)
        .bind(container)
        .execute(pool)
        .await
        .expect("make_list");
        id
    }

    async fn make_item(pool: &SqlitePool, list_id: &str, title: &str) -> String {
        let id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO items (id, list_id, title) VALUES (?, ?, ?)")
            .bind(&id)
            .bind(list_id)
            .bind(title)
            .execute(pool)
            .await
            .expect("make_item");
        id
    }

    async fn make_container(pool: &SqlitePool, user_id: &str, name: &str) -> String {
        let id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO containers (id, user_id, name) VALUES (?, ?, ?)")
            .bind(&id)
            .bind(user_id)
            .bind(name)
            .execute(pool)
            .await
            .expect("make_container");
        id
    }

    async fn make_child_container(
        pool: &SqlitePool,
        user_id: &str,
        name: &str,
        parent: &str,
    ) -> String {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO containers (id, user_id, name, parent_container_id) VALUES (?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(user_id)
        .bind(name)
        .bind(parent)
        .execute(pool)
        .await
        .expect("make_child_container");
        id
    }

    async fn make_comment(
        pool: &SqlitePool,
        user_id: &str,
        entity_type: &str,
        entity_id: &str,
        content: &str,
    ) -> String {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO comments (id, entity_type, entity_id, content, user_id) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(entity_type)
        .bind(entity_id)
        .bind(content)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("make_comment");
        id
    }

    async fn queue_len(pool: &SqlitePool, user_id: &str) -> i64 {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM pending_deletions WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(pool)
                .await
                .unwrap();
        row.0
    }

    async fn row_exists(pool: &SqlitePool, table: &str, id: &str) -> bool {
        let row: Option<(i64,)> = sqlx::query_as(&format!("SELECT 1 FROM {table} WHERE id = ?"))
            .bind(id)
            .fetch_optional(pool)
            .await
            .unwrap();
        row.is_some()
    }

    fn key(kind: DeletableEntity, id: &str) -> PendingDeletionKey {
        PendingDeletionKey {
            entity_type: kind,
            entity_id: id.to_string(),
        }
    }

    #[tokio::test]
    async fn toggle_marks_then_unmarks() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = make_list(&pool, &uid, "Doomed", None, None).await;

        assert!(
            toggle(&pool, &uid, DeletableEntity::List, &list_id)
                .await
                .unwrap()
        );
        assert_eq!(queue_len(&pool, &uid).await, 1);

        assert!(
            !toggle(&pool, &uid, DeletableEntity::List, &list_id)
                .await
                .unwrap()
        );
        assert_eq!(queue_len(&pool, &uid).await, 0);

        assert!(
            row_exists(&pool, "lists", &list_id).await,
            "marking must never delete the entity"
        );
    }

    #[tokio::test]
    async fn toggle_foreign_or_missing_entity_is_not_found() {
        let pool = test_pool().await;
        let owner = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let list_id = make_list(&pool, &owner, "Mine", None, None).await;
        let item_id = make_item(&pool, &list_id, "Mine too").await;
        let container_id = make_container(&pool, &owner, "My crate").await;
        let comment_id = make_comment(&pool, &owner, "list", &list_id, "mine").await;

        for (kind, id, label) in [
            (DeletableEntity::Item, &item_id, "item"),
            (DeletableEntity::List, &list_id, "list"),
            (DeletableEntity::Container, &container_id, "container"),
            (DeletableEntity::Comment, &comment_id, "comment"),
        ] {
            let err = toggle(&pool, &other, kind, id).await.unwrap_err();
            assert!(
                matches!(err, DomainError::NotFound(kind) if kind == label),
                "foreign {label}: got {err:?}"
            );

            let missing = Uuid::new_v4().to_string();
            let err = toggle(&pool, &owner, kind, &missing).await.unwrap_err();
            assert!(
                matches!(err, DomainError::NotFound(kind) if kind == label),
                "missing {label}: got {err:?}"
            );
        }
        assert_eq!(queue_len(&pool, &owner).await, 0);
    }

    #[tokio::test]
    async fn list_all_resolves_label_context_and_href_for_every_type() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let container = make_container(&pool, &uid, "Crate").await;
        let list_id = make_list(&pool, &uid, "Packing", None, Some(&container)).await;
        let item_id = make_item(&pool, &list_id, "Passport").await;
        let comment_id = make_comment(&pool, &uid, "item", &item_id, "don't forget").await;

        for (kind, id) in [
            (DeletableEntity::Item, &item_id),
            (DeletableEntity::List, &list_id),
            (DeletableEntity::Comment, &comment_id),
            (DeletableEntity::Container, &container),
        ] {
            toggle(&pool, &uid, kind, id).await.unwrap();
        }

        let items = list_all(&pool, &uid).await.unwrap();
        assert_eq!(items.len(), 4);
        let find = |kind: DeletableEntity, id: &str| {
            items
                .iter()
                .find(|item| item.entity_type == kind && item.entity_id == id)
                .unwrap_or_else(|| panic!("missing {kind} {id}"))
        };

        let item = find(DeletableEntity::Item, &item_id);
        assert_eq!(item.label, "Passport");
        assert_eq!(item.context.as_deref(), Some("Packing"));
        assert_eq!(
            item.href.as_deref(),
            Some(format!("/lists/{list_id}/items/{item_id}").as_str())
        );
        assert!(item.contains.is_none());
        assert!(!item.marked_at.is_empty());

        let list = find(DeletableEntity::List, &list_id);
        assert_eq!(list.label, "Packing");
        assert_eq!(list.context.as_deref(), Some("Crate"));
        assert_eq!(
            list.href.as_deref(),
            Some(format!("/lists/{list_id}").as_str())
        );
        let contains = list.contains.as_ref().expect("list has a preview");
        assert_eq!(
            (contains.lists, contains.containers, contains.items),
            (0, 0, 1)
        );

        let container_item = find(DeletableEntity::Container, &container);
        assert_eq!(container_item.label, "Crate");
        assert!(container_item.context.is_none(), "root has no context");
        assert_eq!(
            container_item.href.as_deref(),
            Some(format!("/containers/{container}").as_str())
        );
        let contains = container_item.contains.as_ref().expect("preview");
        assert_eq!(
            (contains.lists, contains.containers, contains.items),
            (1, 0, 1)
        );

        let comment = find(DeletableEntity::Comment, &comment_id);
        assert_eq!(comment.label, "don't forget");
        assert_eq!(comment.context.as_deref(), Some("Passport"));
        assert_eq!(
            comment.href.as_deref(),
            Some(format!("/lists/{list_id}/items/{item_id}").as_str())
        );
        assert!(comment.contains.is_none());
    }

    #[tokio::test]
    async fn list_all_context_for_sublist_and_nested_container() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let parent = make_list(&pool, &uid, "Parent", None, None).await;
        let child = make_list(&pool, &uid, "Child", Some(&parent), None).await;
        let outer = make_container(&pool, &uid, "Outer").await;
        let inner = make_child_container(&pool, &uid, "Inner", &outer).await;

        toggle(&pool, &uid, DeletableEntity::List, &parent)
            .await
            .unwrap();
        toggle(&pool, &uid, DeletableEntity::List, &child)
            .await
            .unwrap();
        toggle(&pool, &uid, DeletableEntity::Container, &outer)
            .await
            .unwrap();
        toggle(&pool, &uid, DeletableEntity::Container, &inner)
            .await
            .unwrap();

        let items = list_all(&pool, &uid).await.unwrap();
        let context_of = |id: &str| {
            items
                .iter()
                .find(|item| item.entity_id == id)
                .and_then(|item| item.context.clone())
        };
        assert_eq!(context_of(&parent), None, "root list has no context");
        assert_eq!(context_of(&child).as_deref(), Some("Parent"));
        assert_eq!(context_of(&outer), None, "root container has no context");
        assert_eq!(context_of(&inner).as_deref(), Some("Outer"));
    }

    #[tokio::test]
    async fn list_all_truncates_multibyte_label() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        // 200 chars of 2- and 3-byte UTF-8, no slicing may land mid-character.
        let long: String = "żółć".chars().cycle().take(200).collect();
        let list_id = make_list(&pool, &uid, &long, None, None).await;
        let item_id = make_item(&pool, &list_id, &long).await;
        let comment_id = make_comment(&pool, &uid, "list", &list_id, &long).await;

        for (kind, id) in [
            (DeletableEntity::List, &list_id),
            (DeletableEntity::Item, &item_id),
            (DeletableEntity::Comment, &comment_id),
        ] {
            toggle(&pool, &uid, kind, id).await.unwrap();
        }

        let items = list_all(&pool, &uid).await.unwrap();
        assert_eq!(items.len(), 3);
        for item in &items {
            assert_eq!(
                item.label.chars().count(),
                MAX_LABEL_CHARS,
                "{:?} label length",
                item.entity_type
            );
            assert!(item.label.ends_with('…'));
        }

        // A label that just fits stays untouched.
        let short = "z".repeat(MAX_LABEL_CHARS);
        let short_list = make_list(&pool, &uid, &short, None, None).await;
        toggle(&pool, &uid, DeletableEntity::List, &short_list)
            .await
            .unwrap();
        let items = list_all(&pool, &uid).await.unwrap();
        let found = items
            .iter()
            .find(|item| item.entity_id == short_list)
            .unwrap();
        assert_eq!(found.label, short, "no ellipsis when the label fits");
    }

    #[tokio::test]
    async fn unmark_removes_rows_without_deleting_entities() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = make_list(&pool, &uid, "Sparred", None, None).await;
        let item_id = make_item(&pool, &list_id, "Spared").await;

        toggle(&pool, &uid, DeletableEntity::List, &list_id)
            .await
            .unwrap();
        toggle(&pool, &uid, DeletableEntity::Item, &item_id)
            .await
            .unwrap();

        let removed = unmark(
            &pool,
            &uid,
            &[
                key(DeletableEntity::List, &list_id),
                key(DeletableEntity::Item, &item_id),
                key(DeletableEntity::Container, "never-queued"),
            ],
        )
        .await
        .unwrap();
        assert_eq!(removed, 2);
        assert_eq!(queue_len(&pool, &uid).await, 0);
        assert!(row_exists(&pool, "lists", &list_id).await);
        assert!(row_exists(&pool, "items", &item_id).await);
    }

    #[tokio::test]
    async fn delete_list_with_items_and_sublists_cascades() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let root = make_list(&pool, &uid, "Root", None, None).await;
        let child = make_list(&pool, &uid, "Child", Some(&root), None).await;
        let item_id = make_item(&pool, &child, "Deep item").await;
        let comment_id = make_comment(&pool, &uid, "item", &item_id, "on the deep item").await;

        toggle(&pool, &uid, DeletableEntity::List, &root)
            .await
            .unwrap();

        let deleted = delete(&pool, &uid, &[key(DeletableEntity::List, &root)])
            .await
            .unwrap();
        assert_eq!(deleted, 1);
        assert!(!row_exists(&pool, "lists", &root).await);
        assert!(!row_exists(&pool, "lists", &child).await);
        assert!(!row_exists(&pool, "items", &item_id).await);
        assert!(!row_exists(&pool, "comments", &comment_id).await);
        assert_eq!(queue_len(&pool, &uid).await, 0);
    }

    #[tokio::test]
    async fn delete_container_with_items_cascades() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let container = make_container(&pool, &uid, "Crate").await;
        let inner = make_child_container(&pool, &uid, "Inner", &container).await;
        let list_id = make_list(&pool, &uid, "Boxed", None, Some(&inner)).await;
        let item_id = make_item(&pool, &list_id, "In a box").await;

        toggle(&pool, &uid, DeletableEntity::Container, &container)
            .await
            .unwrap();

        let deleted = delete(&pool, &uid, &[key(DeletableEntity::Container, &container)])
            .await
            .unwrap();
        assert_eq!(deleted, 1);
        assert!(!row_exists(&pool, "containers", &container).await);
        assert!(!row_exists(&pool, "containers", &inner).await);
        assert!(!row_exists(&pool, "lists", &list_id).await);
        assert!(!row_exists(&pool, "items", &item_id).await);
    }

    #[tokio::test]
    async fn delete_key_outside_the_queue_is_not_found_and_deletes_nothing() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let queued = make_list(&pool, &uid, "Queued", None, None).await;
        let not_queued = make_list(&pool, &uid, "Not queued", None, None).await;

        toggle(&pool, &uid, DeletableEntity::List, &queued)
            .await
            .unwrap();

        let err = delete(
            &pool,
            &uid,
            &[
                key(DeletableEntity::List, &queued),
                key(DeletableEntity::List, &not_queued),
            ],
        )
        .await
        .unwrap_err();
        assert!(matches!(err, DomainError::NotFound("pending_deletion")));
        assert!(row_exists(&pool, "lists", &queued).await);
        assert!(row_exists(&pool, "lists", &not_queued).await);
        assert_eq!(queue_len(&pool, &uid).await, 1);

        // Another user's key is not in this user's queue either.
        let other = create_test_user(&pool).await;
        let err = delete(&pool, &other, &[key(DeletableEntity::List, &queued)])
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound("pending_deletion")));
        assert!(row_exists(&pool, "lists", &queued).await);
    }

    #[tokio::test]
    async fn delete_all_counts_only_what_was_actually_deleted() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = make_list(&pool, &uid, "Parent", None, None).await;
        let nested_item = make_item(&pool, &list_id, "Nested item").await;
        let other_list = make_list(&pool, &uid, "Standalone", None, None).await;
        let other_item = make_item(&pool, &other_list, "Standalone item").await;

        // The item is queued as well as the list that holds it.
        for (kind, id) in [
            (DeletableEntity::List, &list_id),
            (DeletableEntity::Item, &nested_item),
            (DeletableEntity::Item, &other_item),
        ] {
            toggle(&pool, &uid, kind, id).await.unwrap();
        }

        let deleted = delete_all(&pool, &uid).await.unwrap();
        assert_eq!(
            deleted, 2,
            "the list (which took the nested item along) and the standalone item; \
             the row the cascade already removed is skipped and not counted"
        );
        assert!(!row_exists(&pool, "lists", &list_id).await);
        assert!(!row_exists(&pool, "items", &nested_item).await);
        assert!(!row_exists(&pool, "items", &other_item).await);
        assert!(row_exists(&pool, "lists", &other_list).await, "not queued");
        assert_eq!(queue_len(&pool, &uid).await, 0);
    }

    #[tokio::test]
    async fn delete_all_on_empty_queue_is_zero() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        assert_eq!(delete_all(&pool, &uid).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn delete_after_triggers_already_cleaned_the_queue_is_skipped() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = make_list(&pool, &uid, "Root", None, None).await;
        let item_id = make_item(&pool, &list_id, "Item").await;

        toggle(&pool, &uid, DeletableEntity::List, &list_id)
            .await
            .unwrap();
        toggle(&pool, &uid, DeletableEntity::Item, &item_id)
            .await
            .unwrap();

        // A row that disappears on its own takes its queue row with it, so the batch
        // below is validated against what is still queued.
        sqlx::query("DELETE FROM items WHERE id = ?")
            .bind(&item_id)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(queue_len(&pool, &uid).await, 1);

        let deleted = delete(&pool, &uid, &[key(DeletableEntity::List, &list_id)])
            .await
            .unwrap();
        assert_eq!(deleted, 1);
        assert!(!row_exists(&pool, "lists", &list_id).await);
    }

    #[tokio::test]
    async fn lists_delete_force_still_rejects_items() {
        // The #223 rule keeps applying outside the queue.
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = make_list(&pool, &uid, "Has items", None, None).await;
        make_item(&pool, &list_id, "Blocker").await;

        let err = crate::lists::delete(&pool, &list_id, &uid, true)
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                DomainError::NotEmpty {
                    kind: "list",
                    items: 1
                }
            ),
            "got {err:?}"
        );
        assert!(row_exists(&pool, "lists", &list_id).await);
    }
}
