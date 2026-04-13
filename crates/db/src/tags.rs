use crate::{DbError, types::TagRow};
use sqlx::{SqliteConnection, SqlitePool};

// ── Constants ─────────────────────────────────────────────────────────────────

const TAG_COLS: &str =
    "id, user_id, name, icon, color, parent_tag_id, tag_type, metadata, created_at";

/// Prefixed version for JOIN queries where tags is aliased as `t`.
const TAG_COLS_T: &str = "t.id, t.user_id, t.name, t.icon, t.color, t.parent_tag_id, t.tag_type, t.metadata, t.created_at";

// ── Input types ───────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct InsertTagInput {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub parent_tag_id: Option<String>,
    pub tag_type: String,
    pub metadata: Option<String>,
}

/// All fields optional. None = leave unchanged. Some(None) = set to NULL.
#[derive(Debug, Default)]
pub struct UpdateTagInput {
    pub name: Option<String>,
    pub icon: Option<Option<String>>,
    pub color: Option<Option<String>>,
    pub parent_tag_id: Option<Option<String>>,
    pub tag_type: Option<String>,
    pub metadata: Option<Option<String>>,
}

// ── Read queries ──────────────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn list_all(pool: &SqlitePool, user_id: &str) -> Result<Vec<TagRow>, DbError> {
    sqlx::query_as::<_, TagRow>(&format!(
        "SELECT {TAG_COLS} FROM tags WHERE user_id = ? ORDER BY name"
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn list_tree(pool: &SqlitePool, user_id: &str) -> Result<Vec<TagRow>, DbError> {
    sqlx::query_as::<_, TagRow>(
        "WITH RECURSIVE tree(\
            id, user_id, name, icon, color, parent_tag_id, tag_type, metadata, created_at, path\
         ) AS (\
            SELECT id, user_id, name, icon, color, parent_tag_id, tag_type, metadata, created_at, name \
            FROM tags WHERE user_id = ? AND parent_tag_id IS NULL \
            UNION ALL \
            SELECT t.id, t.user_id, t.name, t.icon, t.color, t.parent_tag_id, \
                   t.tag_type, t.metadata, t.created_at, tree.path || '/' || t.name \
            FROM tags t JOIN tree ON tree.id = t.parent_tag_id AND t.user_id = ? \
         ) \
         SELECT id, user_id, name, icon, color, parent_tag_id, tag_type, metadata, created_at \
         FROM tree ORDER BY path",
    )
    .bind(user_id)
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
) -> Result<Option<TagRow>, DbError> {
    sqlx::query_as::<_, TagRow>(&format!(
        "SELECT {TAG_COLS} FROM tags WHERE id = ? AND user_id = ?"
    ))
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Returns IDs of all ancestors of `tag_id`, walking up via parent_tag_id.
/// Empty vec if tag has no parent. Used for cycle detection before re-parenting.
/// Only traverses tags belonging to `user_id` (anchor filter prevents cross-user graph leakage).
#[tracing::instrument(skip(pool))]
pub async fn get_ancestors(
    pool: &SqlitePool,
    tag_id: &str,
    user_id: &str,
) -> Result<Vec<String>, DbError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "WITH RECURSIVE anc(id) AS (\
            SELECT parent_tag_id FROM tags WHERE id = ? AND user_id = ? \
            UNION ALL \
            SELECT t.parent_tag_id FROM tags t INNER JOIN anc ON t.id = anc.id \
            WHERE anc.id IS NOT NULL \
         ) \
         SELECT id FROM anc WHERE id IS NOT NULL",
    )
    .bind(tag_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

// ── Tag link read queries ─────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn get_tags_for_item(
    pool: &SqlitePool,
    item_id: &str,
    user_id: &str,
) -> Result<Vec<TagRow>, DbError> {
    sqlx::query_as::<_, TagRow>(&format!(
        "SELECT {TAG_COLS_T} FROM tags t \
         JOIN item_tags it ON it.tag_id = t.id \
         JOIN items i ON i.id = it.item_id \
         JOIN lists l ON l.id = i.list_id \
         WHERE it.item_id = ? AND l.user_id = ? \
         ORDER BY t.name"
    ))
    .bind(item_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn get_tags_for_list(
    pool: &SqlitePool,
    list_id: &str,
    user_id: &str,
) -> Result<Vec<TagRow>, DbError> {
    sqlx::query_as::<_, TagRow>(&format!(
        "SELECT {TAG_COLS_T} FROM tags t \
         JOIN list_tags lt ON lt.tag_id = t.id \
         JOIN lists l ON l.id = lt.list_id \
         WHERE lt.list_id = ? AND l.user_id = ? \
         ORDER BY t.name"
    ))
    .bind(list_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn get_tags_for_container(
    pool: &SqlitePool,
    container_id: &str,
    user_id: &str,
) -> Result<Vec<TagRow>, DbError> {
    sqlx::query_as::<_, TagRow>(&format!(
        "SELECT {TAG_COLS_T} FROM tags t \
         JOIN container_tags ct ON ct.tag_id = t.id \
         JOIN containers c ON c.id = ct.container_id \
         WHERE ct.container_id = ? AND c.user_id = ? \
         ORDER BY t.name"
    ))
    .bind(container_id)
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

/// Returns the first tag of `tag_type` already linked to `item_id`.
/// Used by domain to enforce exclusive types (e.g., at most one "priority" tag per item).
#[tracing::instrument(skip(pool))]
pub async fn get_exclusive_type_tag_for_item(
    pool: &SqlitePool,
    item_id: &str,
    tag_type: &str,
) -> Result<Option<TagRow>, DbError> {
    sqlx::query_as::<_, TagRow>(&format!(
        "SELECT {TAG_COLS_T} FROM tags t \
         JOIN item_tags it ON it.tag_id = t.id \
         WHERE it.item_id = ? AND t.tag_type = ? \
         LIMIT 1"
    ))
    .bind(item_id)
    .bind(tag_type)
    .fetch_optional(pool)
    .await
    .map_err(DbError::Sqlx)
}

// ── Write queries ─────────────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn insert(pool: &SqlitePool, input: &InsertTagInput) -> Result<TagRow, DbError> {
    sqlx::query_as::<_, TagRow>(&format!(
        "INSERT INTO tags (id, user_id, name, icon, color, parent_tag_id, tag_type, metadata) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
         RETURNING {TAG_COLS}"
    ))
    .bind(&input.id)
    .bind(&input.user_id)
    .bind(&input.name)
    .bind(&input.icon)
    .bind(&input.color)
    .bind(&input.parent_tag_id)
    .bind(&input.tag_type)
    .bind(&input.metadata)
    .fetch_one(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[tracing::instrument(skip(pool))]
pub async fn update(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
    input: &UpdateTagInput,
) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "UPDATE tags SET \
            name          = COALESCE(?, name), \
            icon          = CASE WHEN ? = 1 THEN ? ELSE icon END, \
            color         = CASE WHEN ? = 1 THEN ? ELSE color END, \
            parent_tag_id = CASE WHEN ? = 1 THEN ? ELSE parent_tag_id END, \
            tag_type      = COALESCE(?, tag_type), \
            metadata      = CASE WHEN ? = 1 THEN ? ELSE metadata END \
         WHERE id = ? AND user_id = ?",
    )
    .bind(input.name.as_deref())
    .bind(input.icon.is_some() as i32)
    .bind(input.icon.as_ref().and_then(|v| v.as_deref()))
    .bind(input.color.is_some() as i32)
    .bind(input.color.as_ref().and_then(|v| v.as_deref()))
    .bind(input.parent_tag_id.is_some() as i32)
    .bind(input.parent_tag_id.as_ref().and_then(|v| v.as_deref()))
    .bind(input.tag_type.as_deref())
    .bind(input.metadata.is_some() as i32)
    .bind(input.metadata.as_ref().and_then(|v| v.as_deref()))
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let rows = sqlx::query("DELETE FROM tags WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

// ── Merge helpers (called inside a transaction) ───────────────────────────────
// ON DELETE CASCADE on item_tags.tag_id, list_tags.tag_id, container_tags.tag_id
// means delete_by_id(source) automatically removes all remaining source link rows.
// reassign_* only need to INSERT OR IGNORE the target copies.

#[tracing::instrument(skip(conn))]
pub async fn reassign_item_links(
    conn: &mut SqliteConnection,
    source_id: &str,
    target_id: &str,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT OR IGNORE INTO item_tags (item_id, tag_id) \
         SELECT item_id, ? FROM item_tags WHERE tag_id = ?",
    )
    .bind(target_id)
    .bind(source_id)
    .execute(&mut *conn)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(())
}

#[tracing::instrument(skip(conn))]
pub async fn reassign_list_links(
    conn: &mut SqliteConnection,
    source_id: &str,
    target_id: &str,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT OR IGNORE INTO list_tags (list_id, tag_id) \
         SELECT list_id, ? FROM list_tags WHERE tag_id = ?",
    )
    .bind(target_id)
    .bind(source_id)
    .execute(&mut *conn)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(())
}

#[tracing::instrument(skip(conn))]
pub async fn reassign_container_links(
    conn: &mut SqliteConnection,
    source_id: &str,
    target_id: &str,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT OR IGNORE INTO container_tags (container_id, tag_id) \
         SELECT container_id, ? FROM container_tags WHERE tag_id = ?",
    )
    .bind(target_id)
    .bind(source_id)
    .execute(&mut *conn)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(())
}

/// Reparent source's children to target before deleting source.
/// Prevents FK violation when deleting a tag that still has children.
#[tracing::instrument(skip(conn))]
pub async fn reparent_children(
    conn: &mut SqliteConnection,
    source_id: &str,
    target_id: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE tags SET parent_tag_id = ? WHERE parent_tag_id = ?")
        .bind(target_id)
        .bind(source_id)
        .execute(&mut *conn)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(())
}

/// Delete without user_id check. Only call inside a merge transaction after
/// ownership of both source and target has already been verified by domain.
#[tracing::instrument(skip(conn))]
pub async fn delete_by_id(conn: &mut SqliteConnection, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM tags WHERE id = ?")
        .bind(id)
        .execute(&mut *conn)
        .await
        .map_err(DbError::Sqlx)?;
    Ok(())
}

// ── Tag link write queries ────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn add_item_tag(
    pool: &SqlitePool,
    item_id: &str,
    tag_id: &str,
    user_id: &str,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT OR IGNORE INTO item_tags (item_id, tag_id) \
         SELECT ?, ? \
         WHERE EXISTS (SELECT 1 FROM items i JOIN lists l ON l.id = i.list_id WHERE i.id = ? AND l.user_id = ?) \
           AND EXISTS (SELECT 1 FROM tags WHERE id = ? AND user_id = ?)",
    )
    .bind(item_id)
    .bind(tag_id)
    .bind(item_id)
    .bind(user_id)
    .bind(tag_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(())
}

#[tracing::instrument(skip(pool))]
pub async fn remove_item_tag(
    pool: &SqlitePool,
    item_id: &str,
    tag_id: &str,
    user_id: &str,
) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "DELETE FROM item_tags WHERE item_id = ? AND tag_id = ? \
         AND EXISTS (SELECT 1 FROM items i JOIN lists l ON l.id = i.list_id WHERE i.id = ? AND l.user_id = ?)",
    )
    .bind(item_id)
    .bind(tag_id)
    .bind(item_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

#[tracing::instrument(skip(pool))]
pub async fn add_list_tag(
    pool: &SqlitePool,
    list_id: &str,
    tag_id: &str,
    user_id: &str,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT OR IGNORE INTO list_tags (list_id, tag_id) \
         SELECT ?, ? \
         WHERE EXISTS (SELECT 1 FROM lists WHERE id = ? AND user_id = ?) \
           AND EXISTS (SELECT 1 FROM tags WHERE id = ? AND user_id = ?)",
    )
    .bind(list_id)
    .bind(tag_id)
    .bind(list_id)
    .bind(user_id)
    .bind(tag_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(())
}

#[tracing::instrument(skip(pool))]
pub async fn remove_list_tag(
    pool: &SqlitePool,
    list_id: &str,
    tag_id: &str,
    user_id: &str,
) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "DELETE FROM list_tags WHERE list_id = ? AND tag_id = ? \
         AND EXISTS (SELECT 1 FROM lists WHERE id = ? AND user_id = ?)",
    )
    .bind(list_id)
    .bind(tag_id)
    .bind(list_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows.rows_affected() > 0)
}

/// Returns all list-tag links for a given user (joined through lists.user_id).
/// Used by home page tag filter bar.
#[tracing::instrument(skip(pool))]
pub async fn get_all_list_tag_links(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<(String, String)>, DbError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT lt.list_id, lt.tag_id \
         FROM list_tags lt \
         JOIN lists l ON l.id = lt.list_id \
         WHERE l.user_id = ?",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(rows)
}

#[tracing::instrument(skip(pool))]
pub async fn add_container_tag(
    pool: &SqlitePool,
    container_id: &str,
    tag_id: &str,
    user_id: &str,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT OR IGNORE INTO container_tags (container_id, tag_id) \
         SELECT ?, ? \
         WHERE EXISTS (SELECT 1 FROM containers WHERE id = ? AND user_id = ?) \
           AND EXISTS (SELECT 1 FROM tags WHERE id = ? AND user_id = ?)",
    )
    .bind(container_id)
    .bind(tag_id)
    .bind(container_id)
    .bind(user_id)
    .bind(tag_id)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(DbError::Sqlx)?;
    Ok(())
}

#[tracing::instrument(skip(pool))]
pub async fn remove_container_tag(
    pool: &SqlitePool,
    container_id: &str,
    tag_id: &str,
    user_id: &str,
) -> Result<bool, DbError> {
    let rows = sqlx::query(
        "DELETE FROM container_tags WHERE container_id = ? AND tag_id = ? \
         AND EXISTS (SELECT 1 FROM containers WHERE id = ? AND user_id = ?)",
    )
    .bind(container_id)
    .bind(tag_id)
    .bind(container_id)
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

    fn new_id() -> String {
        Uuid::new_v4().to_string()
    }

    async fn insert_tag(
        pool: &SqlitePool,
        user_id: &str,
        name: &str,
        parent: Option<&str>,
        tag_type: &str,
    ) -> TagRow {
        insert(
            pool,
            &InsertTagInput {
                id: new_id(),
                user_id: user_id.to_string(),
                name: name.to_string(),
                icon: None,
                color: None,
                parent_tag_id: parent.map(str::to_string),
                tag_type: tag_type.to_string(),
                metadata: None,
            },
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn insert_and_get_one() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let tag = insert_tag(&pool, &uid, "Work", None, "tag").await;

        assert!(!tag.id.is_empty());
        assert_eq!(tag.name, "Work");
        assert_eq!(tag.tag_type, "tag");
        assert!(tag.parent_tag_id.is_none());

        let found = get_one(&pool, &tag.id, &uid).await.unwrap();
        assert_eq!(found.unwrap().name, "Work");
    }

    #[tokio::test]
    async fn get_one_wrong_user_returns_none() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let tag = insert_tag(&pool, &uid, "Work", None, "tag").await;

        let found = get_one(&pool, &tag.id, &other).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn list_all_sorted_by_name() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        insert_tag(&pool, &uid, "Zorro", None, "tag").await;
        insert_tag(&pool, &uid, "Alpha", None, "tag").await;
        insert_tag(&pool, &uid, "Medium", None, "tag").await;

        let tags = list_all(&pool, &uid).await.unwrap();
        let names: Vec<&str> = tags.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["Alpha", "Medium", "Zorro"]);
    }

    #[tokio::test]
    async fn list_tree_depth_first_order() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let root = insert_tag(&pool, &uid, "Root", None, "tag").await;
        let child = insert_tag(&pool, &uid, "Child", Some(&root.id), "tag").await;
        insert_tag(&pool, &uid, "Grandchild", Some(&child.id), "tag").await;
        insert_tag(&pool, &uid, "Sibling", Some(&root.id), "tag").await;

        let tree = list_tree(&pool, &uid).await.unwrap();
        assert_eq!(tree.len(), 4);
        assert_eq!(tree[0].name, "Root");
        assert_eq!(tree[1].name, "Child");
        assert_eq!(tree[2].name, "Grandchild");
        assert_eq!(tree[3].name, "Sibling");
    }

    #[tokio::test]
    async fn get_ancestors_chain() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let country = insert_tag(&pool, &uid, "Poland", None, "country").await;
        let city = insert_tag(&pool, &uid, "Warsaw", Some(&country.id), "city").await;
        let addr = insert_tag(&pool, &uid, "Main St", Some(&city.id), "address").await;

        let ancestors = get_ancestors(&pool, &addr.id, &uid).await.unwrap();
        assert_eq!(ancestors.len(), 2);
        assert!(ancestors.contains(&city.id));
        assert!(ancestors.contains(&country.id));

        assert!(get_ancestors(&pool, &country.id, &uid).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn update_name_and_color() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let tag = insert_tag(&pool, &uid, "Work", None, "tag").await;

        let changed = update(
            &pool,
            &tag.id,
            &uid,
            &UpdateTagInput {
                name: Some("Personal".to_string()),
                color: Some(Some("#ff0000".to_string())),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert!(changed);

        let found = get_one(&pool, &tag.id, &uid).await.unwrap().unwrap();
        assert_eq!(found.name, "Personal");
        assert_eq!(found.color.as_deref(), Some("#ff0000"));
    }

    #[tokio::test]
    async fn update_clear_color_to_null() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let tag = insert(
            &pool,
            &InsertTagInput {
                id: new_id(),
                user_id: uid.clone(),
                name: "Tag".into(),
                icon: None,
                color: Some("#fff".into()),
                parent_tag_id: None,
                tag_type: "tag".into(),
                metadata: None,
            },
        )
        .await
        .unwrap();

        update(
            &pool,
            &tag.id,
            &uid,
            &UpdateTagInput {
                color: Some(None),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let found = get_one(&pool, &tag.id, &uid).await.unwrap().unwrap();
        assert!(found.color.is_none());
    }

    #[tokio::test]
    async fn delete_removes_tag() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let tag = insert_tag(&pool, &uid, "ToDelete", None, "tag").await;

        assert!(delete(&pool, &tag.id, &uid).await.unwrap());
        assert!(get_one(&pool, &tag.id, &uid).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn merge_helpers_reassign_and_cascade() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let source = insert_tag(&pool, &uid, "Source", None, "tag").await;
        let target = insert_tag(&pool, &uid, "Target", None, "tag").await;
        let list_id = new_id();
        let item_id = new_id();

        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES (?, ?, 'L')")
            .bind(&list_id)
            .bind(&uid)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO items (id, list_id, title) VALUES (?, ?, 'I')")
            .bind(&item_id)
            .bind(&list_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO item_tags (item_id, tag_id) VALUES (?, ?)")
            .bind(&item_id)
            .bind(&source.id)
            .execute(&pool)
            .await
            .unwrap();

        let mut tx = pool.begin().await.unwrap();
        reassign_item_links(&mut tx, &source.id, &target.id)
            .await
            .unwrap();
        delete_by_id(&mut tx, &source.id).await.unwrap();
        tx.commit().await.unwrap();

        let rows: Vec<(String,)> = sqlx::query_as("SELECT tag_id FROM item_tags WHERE item_id = ?")
            .bind(&item_id)
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, target.id);

        assert!(get_one(&pool, &source.id, &uid).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn add_and_remove_item_tag() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let tag = insert_tag(&pool, &uid, "Work", None, "tag").await;
        let list_id = new_id();
        let item_id = new_id();

        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES (?, ?, 'L')")
            .bind(&list_id)
            .bind(&uid)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO items (id, list_id, title) VALUES (?, ?, 'I')")
            .bind(&item_id)
            .bind(&list_id)
            .execute(&pool)
            .await
            .unwrap();

        add_item_tag(&pool, &item_id, &tag.id, &uid).await.unwrap();
        let tags = get_tags_for_item(&pool, &item_id, &uid).await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "Work");

        assert!(
            remove_item_tag(&pool, &item_id, &tag.id, &uid)
                .await
                .unwrap()
        );
        assert!(
            get_tags_for_item(&pool, &item_id, &uid)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn add_item_tag_wrong_user_is_noop() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let other = create_test_user(&pool).await;
        let tag = insert_tag(&pool, &uid, "Work", None, "tag").await;
        let list_id = new_id();
        let item_id = new_id();

        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES (?, ?, 'L')")
            .bind(&list_id)
            .bind(&uid)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO items (id, list_id, title) VALUES (?, ?, 'I')")
            .bind(&item_id)
            .bind(&list_id)
            .execute(&pool)
            .await
            .unwrap();

        add_item_tag(&pool, &item_id, &tag.id, &other)
            .await
            .unwrap();
        assert!(
            get_tags_for_item(&pool, &item_id, &uid)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn get_all_list_tag_links_returns_user_links() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let tag = insert_tag(&pool, &uid, "Work", None, "tag").await;
        let list_id = new_id();

        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES (?, ?, 'L')")
            .bind(&list_id)
            .bind(&uid)
            .execute(&pool)
            .await
            .unwrap();

        add_list_tag(&pool, &list_id, &tag.id, &uid).await.unwrap();

        let links: Vec<(String, String)> = get_all_list_tag_links(&pool, &uid).await.unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].0, list_id);
        assert_eq!(links[0].1, tag.id);
    }

    #[tokio::test]
    async fn get_exclusive_type_tag_returns_existing() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let p_tag = insert_tag(&pool, &uid, "High", None, "priority").await;
        let list_id = new_id();
        let item_id = new_id();

        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES (?, ?, 'L')")
            .bind(&list_id)
            .bind(&uid)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO items (id, list_id, title) VALUES (?, ?, 'I')")
            .bind(&item_id)
            .bind(&list_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO item_tags (item_id, tag_id) VALUES (?, ?)")
            .bind(&item_id)
            .bind(&p_tag.id)
            .execute(&pool)
            .await
            .unwrap();

        let found = get_exclusive_type_tag_for_item(&pool, &item_id, "priority")
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, p_tag.id);

        let none = get_exclusive_type_tag_for_item(&pool, &item_id, "tag")
            .await
            .unwrap();
        assert!(none.is_none());
    }
}
