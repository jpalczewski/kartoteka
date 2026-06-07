use crate::{DomainError, rules};
use kartoteka_db::{
    self as db,
    tags::{InsertTagInput, UpdateTagInput},
};
use kartoteka_shared::{FilterMode, dto::responses::HomeFilterResult};
use serde::{Deserialize, Serialize};
use sqlx::{QueryBuilder, SqlitePool};
use uuid::Uuid;

// ── Public domain types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub parent_tag_id: Option<String>,
    pub tag_type: String,
    pub metadata: Option<String>,
    pub created_at: String,
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub parent_tag_id: Option<String>,
    /// Defaults to "tag" if None.
    pub tag_type: Option<String>,
    pub metadata: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTagRequest {
    pub name: Option<String>,
    pub icon: Option<Option<String>>,
    pub color: Option<Option<String>>,
    /// None = don't change. Some(None) = clear parent. Some(Some(id)) = set new parent.
    pub parent_tag_id: Option<Option<String>>,
    pub tag_type: Option<String>,
    pub metadata: Option<Option<String>>,
}

// ── Conversion ────────────────────────────────────────────────────────────────

fn row_to_tag(row: db::types::TagRow) -> Tag {
    Tag {
        id: row.id,
        user_id: row.user_id,
        name: row.name,
        icon: row.icon,
        color: row.color,
        parent_tag_id: row.parent_tag_id,
        tag_type: row.tag_type,
        metadata: row.metadata,
        created_at: row.created_at,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn push_ids<'a>(qb: &mut QueryBuilder<'a, sqlx::Sqlite>, ids: &'a [String]) {
    let mut sep = qb.separated(", ");
    for id in ids {
        sep.push_bind(id);
    }
}

fn dedup_strings(items: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    items
        .into_iter()
        .filter(|id| seen.insert(id.clone()))
        .collect()
}

// ── Public functions ──────────────────────────────────────────────────────────

async fn listwise_matching(
    pool: &SqlitePool,
    user_id: &str,
    tag_ids: &[String],
) -> Result<Vec<String>, DomainError> {
    if tag_ids.is_empty() {
        return Ok(vec![]);
    }
    let mut qb = QueryBuilder::<sqlx::Sqlite>::new(
        "SELECT lt.list_id FROM list_tags lt \
         JOIN lists l ON lt.list_id = l.id \
         WHERE l.user_id = ",
    );
    qb.push_bind(user_id);
    qb.push(" AND lt.tag_id IN (");
    push_ids(&mut qb, tag_ids);
    qb.push(") GROUP BY lt.list_id HAVING COUNT(DISTINCT lt.tag_id) = ");
    qb.push_bind(tag_ids.len() as i64);
    Ok(qb
        .build_query_scalar()
        .fetch_all(pool)
        .await
        .map_err(db::DbError::Sqlx)?)
}

async fn list_co_occurring_tags(
    pool: &SqlitePool,
    list_ids: &[String],
    exclude_tag_ids: &[String],
) -> Result<Vec<String>, DomainError> {
    if list_ids.is_empty() {
        return Ok(vec![]);
    }
    let mut qb = QueryBuilder::<sqlx::Sqlite>::new(
        "SELECT DISTINCT tag_id FROM list_tags WHERE list_id IN (",
    );
    push_ids(&mut qb, list_ids);
    qb.push(")");
    if !exclude_tag_ids.is_empty() {
        qb.push(" AND tag_id NOT IN (");
        push_ids(&mut qb, exclude_tag_ids);
        qb.push(")");
    }
    Ok(qb
        .build_query_scalar()
        .fetch_all(pool)
        .await
        .map_err(db::DbError::Sqlx)?)
}

async fn itemwise_matching(
    pool: &SqlitePool,
    user_id: &str,
    tag_ids: &[String],
) -> Result<Vec<String>, DomainError> {
    if tag_ids.is_empty() {
        return Ok(vec![]);
    }
    let mut qb = QueryBuilder::<sqlx::Sqlite>::new(
        "SELECT list_id FROM (\
            SELECT i.list_id, COUNT(DISTINCT it.tag_id) AS covered \
            FROM items i \
            JOIN item_tags it ON i.id = it.item_id \
            JOIN lists l ON i.list_id = l.id \
            WHERE l.user_id = ",
    );
    qb.push_bind(user_id);
    qb.push(" AND it.tag_id IN (");
    push_ids(&mut qb, tag_ids);
    qb.push(") GROUP BY i.list_id) WHERE covered = ");
    qb.push_bind(tag_ids.len() as i64);
    Ok(qb
        .build_query_scalar()
        .fetch_all(pool)
        .await
        .map_err(db::DbError::Sqlx)?)
}

async fn container_ids_for_lists(
    pool: &SqlitePool,
    list_ids: &[String],
) -> Result<Vec<String>, DomainError> {
    if list_ids.is_empty() {
        return Ok(vec![]);
    }
    let mut qb =
        QueryBuilder::<sqlx::Sqlite>::new("SELECT DISTINCT container_id FROM lists WHERE id IN (");
    push_ids(&mut qb, list_ids);
    qb.push(") AND container_id IS NOT NULL");
    Ok(qb
        .build_query_scalar()
        .fetch_all(pool)
        .await
        .map_err(db::DbError::Sqlx)?)
}

async fn item_co_occurring_tags(
    pool: &SqlitePool,
    list_ids: &[String],
    exclude_tag_ids: &[String],
) -> Result<Vec<String>, DomainError> {
    if list_ids.is_empty() {
        return Ok(vec![]);
    }
    let mut qb = QueryBuilder::<sqlx::Sqlite>::new(
        "SELECT DISTINCT it.tag_id FROM item_tags it \
         JOIN items i ON i.id = it.item_id \
         WHERE i.list_id IN (",
    );
    push_ids(&mut qb, list_ids);
    qb.push(")");
    if !exclude_tag_ids.is_empty() {
        qb.push(" AND it.tag_id NOT IN (");
        push_ids(&mut qb, exclude_tag_ids);
        qb.push(")");
    }
    Ok(qb
        .build_query_scalar()
        .fetch_all(pool)
        .await
        .map_err(db::DbError::Sqlx)?)
}

pub async fn filter_home_by_tags(
    pool: &SqlitePool,
    user_id: &str,
    tag_ids: &[String],
    mode: FilterMode,
) -> Result<HomeFilterResult, DomainError> {
    if tag_ids.is_empty() {
        return Ok(HomeFilterResult::default());
    }
    match mode {
        FilterMode::Listwise => {
            let list_ids = listwise_matching(pool, user_id, tag_ids).await?;
            let matching_container_ids = container_ids_for_lists(pool, &list_ids).await?;
            let related_tag_ids = list_co_occurring_tags(pool, &list_ids, tag_ids).await?;
            Ok(HomeFilterResult {
                matching_list_ids: list_ids,
                matching_container_ids,
                related_tag_ids,
            })
        }
        FilterMode::Itemwise => {
            let list_ids = itemwise_matching(pool, user_id, tag_ids).await?;
            let matching_container_ids = container_ids_for_lists(pool, &list_ids).await?;
            let related_tag_ids = item_co_occurring_tags(pool, &list_ids, tag_ids).await?;
            Ok(HomeFilterResult {
                matching_list_ids: list_ids,
                matching_container_ids,
                related_tag_ids,
            })
        }
        FilterMode::Joined => {
            let list_ids_l = listwise_matching(pool, user_id, tag_ids).await?;
            let list_ids_i = itemwise_matching(pool, user_id, tag_ids).await?;
            let list_ids = dedup_strings(list_ids_l.iter().chain(list_ids_i.iter()).cloned());
            let matching_container_ids = container_ids_for_lists(pool, &list_ids).await?;
            let mut related = list_co_occurring_tags(pool, &list_ids_l, tag_ids).await?;
            related.extend(item_co_occurring_tags(pool, &list_ids_i, tag_ids).await?);
            let related_tag_ids = dedup_strings(related);
            Ok(HomeFilterResult {
                matching_list_ids: list_ids,
                matching_container_ids,
                related_tag_ids,
            })
        }
    }
}

#[tracing::instrument(skip(pool))]
pub async fn list_all(pool: &SqlitePool, user_id: &str) -> Result<Vec<Tag>, DomainError> {
    Ok(db::tags::list_all(pool, user_id)
        .await?
        .into_iter()
        .map(row_to_tag)
        .collect())
}

#[tracing::instrument(skip(pool))]
pub async fn list_tree(pool: &SqlitePool, user_id: &str) -> Result<Vec<Tag>, DomainError> {
    Ok(db::tags::list_tree(pool, user_id)
        .await?
        .into_iter()
        .map(row_to_tag)
        .collect())
}

#[tracing::instrument(skip(pool))]
pub async fn get_one(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<Tag>, DomainError> {
    Ok(db::tags::get_one(pool, id, user_id).await?.map(row_to_tag))
}

/// Validates tag name and optional color without pool access.
/// Shared by `create` and the `create_tags` MCP batch handler.
pub fn validate_tag_input(name: &str, color: Option<&str>) -> Result<(), DomainError> {
    rules::tags::validate_name(name)?;
    if let Some(c) = color {
        rules::tags::validate_color(c)?;
    }
    Ok(())
}

#[tracing::instrument(skip(pool))]
pub async fn create(
    pool: &SqlitePool,
    user_id: &str,
    req: &CreateTagRequest,
) -> Result<Tag, DomainError> {
    validate_tag_input(&req.name, req.color.as_deref())?;

    // Phase 2: THINK
    let tag_type = req.tag_type.as_deref().unwrap_or("tag");
    DomainError::ensure_unique(
        db::tags::find_id_by_name_in_scope(
            pool,
            user_id,
            &req.name,
            req.parent_tag_id.as_deref(),
            None,
        )
        .await?,
        "tag",
    )?;

    // Phase 3: WRITE
    let row = db::tags::insert(
        pool,
        &InsertTagInput {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            name: req.name.clone(),
            icon: req.icon.clone(),
            color: req.color.clone(),
            parent_tag_id: req.parent_tag_id.clone(),
            tag_type: tag_type.to_string(),
            metadata: req.metadata.clone(),
        },
    )
    .await?;
    Ok(row_to_tag(row))
}

#[tracing::instrument(skip(pool))]
pub async fn update(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
    req: &UpdateTagRequest,
) -> Result<Option<Tag>, DomainError> {
    // Phase 1: READ — get current state (needed for hierarchy re-checks)
    let current = match db::tags::get_one(pool, id, user_id).await? {
        Some(t) => t,
        None => return Ok(None),
    };

    // Phase 2: THINK — validate parent change
    if let Some(Some(new_parent_id)) = &req.parent_tag_id {
        // Cycle detection
        let ancestors = db::tags::get_ancestors(pool, new_parent_id, user_id).await?;
        rules::tags::validate_parent(id, new_parent_id, &ancestors)?;
    }

    // Duplicate name check — use effective parent after any parent change
    if let Some(ref new_name) = req.name {
        let effective_parent = match &req.parent_tag_id {
            Some(p) => p.as_deref(),
            None => current.parent_tag_id.as_deref(),
        };
        DomainError::ensure_unique(
            db::tags::find_id_by_name_in_scope(pool, user_id, new_name, effective_parent, Some(id))
                .await?,
            "tag",
        )?;
    }

    // Phase 3: WRITE
    let updated = db::tags::update(
        pool,
        id,
        user_id,
        &UpdateTagInput {
            name: req.name.clone(),
            icon: req.icon.clone(),
            color: req.color.clone(),
            parent_tag_id: req.parent_tag_id.clone(),
            tag_type: req.tag_type.clone(),
            metadata: req.metadata.clone(),
        },
    )
    .await?;

    if !updated {
        return Ok(None);
    }

    let row = db::tags::get_one(pool, id, user_id)
        .await?
        .ok_or(DomainError::NotFound("tag"))?;
    Ok(Some(row_to_tag(row)))
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, user_id: &str, id: &str) -> Result<bool, DomainError> {
    Ok(db::tags::delete(pool, id, user_id).await?)
}

/// Merge `source` into `target`: reassign all links + children, then delete source.
/// Both tags must belong to `user_id`. Returns the target tag after merge.
#[tracing::instrument(skip(pool))]
pub async fn merge(
    pool: &SqlitePool,
    user_id: &str,
    source_id: &str,
    target_id: &str,
) -> Result<Tag, DomainError> {
    // Phase 1: READ — verify ownership of both tags
    let _source = db::tags::get_one(pool, source_id, user_id)
        .await?
        .ok_or(DomainError::NotFound("source_tag"))?;
    let _target = db::tags::get_one(pool, target_id, user_id)
        .await?
        .ok_or(DomainError::NotFound("target_tag"))?;

    // Phase 2: THINK
    rules::tags::validate_merge(source_id, target_id)?;

    // Phase 3: WRITE — transaction keeps everything consistent
    let mut tx = pool.begin().await.map_err(db::DbError::Sqlx)?;
    db::tags::reassign_item_links(&mut tx, source_id, target_id).await?;
    db::tags::reassign_list_links(&mut tx, source_id, target_id).await?;
    db::tags::reassign_container_links(&mut tx, source_id, target_id).await?;
    db::tags::reparent_children(&mut tx, source_id, target_id).await?;
    db::tags::delete_by_id(&mut tx, source_id).await?;
    tx.commit().await.map_err(db::DbError::Sqlx)?;

    let row = db::tags::get_one(pool, target_id, user_id)
        .await?
        .ok_or(DomainError::NotFound("tag"))?;
    Ok(row_to_tag(row))
}

// ── Tag link operations ───────────────────────────────────────────────────────

/// Assign a tag to an item. Enforces exclusive type constraint (e.g. one "priority" per item).
#[tracing::instrument(skip(pool))]
pub async fn assign_to_item(
    pool: &SqlitePool,
    user_id: &str,
    item_id: &str,
    tag_id: &str,
) -> Result<(), DomainError> {
    let tag = db::tags::get_one(pool, tag_id, user_id)
        .await?
        .ok_or(DomainError::NotFound("tag"))?;
    let existing = db::tags::get_exclusive_type_tag_for_item(pool, item_id, &tag.tag_type).await?;
    rules::tags::validate_exclusive_type(&tag.tag_type, existing.as_ref().map(|t| t.id.as_str()))?;
    if !db::tags::add_item_tag(pool, item_id, tag_id, user_id).await? {
        return Err(DomainError::NotFound("item_or_tag"));
    }
    Ok(())
}

#[tracing::instrument(skip(pool))]
pub async fn remove_from_item(
    pool: &SqlitePool,
    user_id: &str,
    item_id: &str,
    tag_id: &str,
) -> Result<bool, DomainError> {
    Ok(db::tags::remove_item_tag(pool, item_id, tag_id, user_id).await?)
}

#[tracing::instrument(skip(pool))]
pub async fn assign_to_list(
    pool: &SqlitePool,
    user_id: &str,
    list_id: &str,
    tag_id: &str,
) -> Result<(), DomainError> {
    if !db::tags::add_list_tag(pool, list_id, tag_id, user_id).await? {
        return Err(DomainError::NotFound("list_or_tag"));
    }
    Ok(())
}

#[tracing::instrument(skip(pool))]
pub async fn remove_from_list(
    pool: &SqlitePool,
    user_id: &str,
    list_id: &str,
    tag_id: &str,
) -> Result<bool, DomainError> {
    Ok(db::tags::remove_list_tag(pool, list_id, tag_id, user_id).await?)
}

#[tracing::instrument(skip(pool))]
pub async fn assign_to_container(
    pool: &SqlitePool,
    user_id: &str,
    container_id: &str,
    tag_id: &str,
) -> Result<(), DomainError> {
    if !db::tags::add_container_tag(pool, container_id, tag_id, user_id).await? {
        return Err(DomainError::NotFound("container_or_tag"));
    }
    Ok(())
}

#[tracing::instrument(skip(pool))]
pub async fn remove_from_container(
    pool: &SqlitePool,
    user_id: &str,
    container_id: &str,
    tag_id: &str,
) -> Result<bool, DomainError> {
    Ok(db::tags::remove_container_tag(pool, container_id, tag_id, user_id).await?)
}

#[tracing::instrument(skip(pool))]
pub async fn get_for_item(
    pool: &SqlitePool,
    user_id: &str,
    item_id: &str,
) -> Result<Vec<Tag>, DomainError> {
    Ok(db::tags::get_tags_for_item(pool, item_id, user_id)
        .await?
        .into_iter()
        .map(row_to_tag)
        .collect())
}

#[tracing::instrument(skip(pool))]
pub async fn get_for_list(
    pool: &SqlitePool,
    user_id: &str,
    list_id: &str,
) -> Result<Vec<Tag>, DomainError> {
    Ok(db::tags::get_tags_for_list(pool, list_id, user_id)
        .await?
        .into_iter()
        .map(row_to_tag)
        .collect())
}

#[tracing::instrument(skip(pool))]
pub async fn get_for_container(
    pool: &SqlitePool,
    user_id: &str,
    container_id: &str,
) -> Result<Vec<Tag>, DomainError> {
    Ok(
        db::tags::get_tags_for_container(pool, container_id, user_id)
            .await?
            .into_iter()
            .map(row_to_tag)
            .collect(),
    )
}

// ── Inverse tag lookup ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct TaggedItem {
    pub id: String,
    pub title: String,
    pub list_id: String,
    pub completed: bool,
}

#[derive(Debug, Serialize)]
pub struct TaggedList {
    pub id: String,
    pub name: String,
    pub container_id: Option<String>,
    pub archived: bool,
}

#[derive(Serialize)]
pub struct TagEntities {
    pub items: Vec<TaggedItem>,
    pub lists: Vec<TaggedList>,
}

#[tracing::instrument(skip(pool))]
pub async fn get_entities_by_tag(
    pool: &SqlitePool,
    user_id: &str,
    tag_id: &str,
    entity_type: Option<&str>,
) -> Result<TagEntities, DomainError> {
    db::tags::get_one(pool, tag_id, user_id)
        .await?
        .ok_or(DomainError::NotFound("tag"))?;

    let (items, lists) = match entity_type {
        Some("item") => (
            db::tags::get_items_by_tag(pool, tag_id, user_id).await?,
            vec![],
        ),
        Some("list") => (
            vec![],
            db::tags::get_lists_by_tag(pool, tag_id, user_id).await?,
        ),
        _ => tokio::try_join!(
            db::tags::get_items_by_tag(pool, tag_id, user_id),
            db::tags::get_lists_by_tag(pool, tag_id, user_id),
        )?,
    };

    Ok(TagEntities {
        items: items
            .into_iter()
            .map(|r| TaggedItem {
                id: r.id,
                title: r.title,
                list_id: r.list_id,
                completed: r.completed,
            })
            .collect(),
        lists: lists
            .into_iter()
            .map(|r| TaggedList {
                id: r.id,
                name: r.name,
                container_id: r.container_id,
                archived: r.archived,
            })
            .collect(),
    })
}

// ── Integration tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{create_test_user, test_pool};

    async fn make_tag(pool: &SqlitePool, uid: &str, name: &str) -> Tag {
        create(
            pool,
            uid,
            &CreateTagRequest {
                name: name.to_string(),
                icon: None,
                color: None,
                parent_tag_id: None,
                tag_type: None,
                metadata: None,
            },
        )
        .await
        .unwrap()
    }

    async fn make_item(pool: &SqlitePool, uid: &str) -> String {
        let lid = Uuid::new_v4().to_string();
        let iid = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES (?, ?, 'L')")
            .bind(&lid)
            .bind(uid)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO items (id, list_id, title) VALUES (?, ?, 'I')")
            .bind(&iid)
            .bind(&lid)
            .execute(pool)
            .await
            .unwrap();
        iid
    }

    #[tokio::test]
    async fn create_and_list_all() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        make_tag(&pool, &uid, "Work").await;
        make_tag(&pool, &uid, "Personal").await;

        let tags = list_all(&pool, &uid).await.unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].name, "Personal"); // sorted
    }

    #[tokio::test]
    async fn update_name() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let tag = make_tag(&pool, &uid, "Work").await;

        let updated = update(
            &pool,
            &uid,
            &tag.id,
            &UpdateTagRequest {
                name: Some("Job".to_string()),
                icon: None,
                color: None,
                parent_tag_id: None,
                tag_type: None,
                metadata: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.unwrap().name, "Job");
    }

    #[tokio::test]
    async fn update_parent_cycle_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let parent = make_tag(&pool, &uid, "Parent").await;
        let child = create(
            &pool,
            &uid,
            &CreateTagRequest {
                name: "Child".to_string(),
                parent_tag_id: Some(parent.id.clone()),
                tag_type: None,
                icon: None,
                color: None,
                metadata: None,
            },
        )
        .await
        .unwrap();

        // Set parent's parent to child — would create a cycle
        let result = update(
            &pool,
            &uid,
            &parent.id,
            &UpdateTagRequest {
                parent_tag_id: Some(Some(child.id.clone())),
                name: None,
                icon: None,
                color: None,
                tag_type: None,
                metadata: None,
            },
        )
        .await;

        assert!(matches!(result.unwrap_err(), DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn delete_tag() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let tag = make_tag(&pool, &uid, "ToDelete").await;

        assert!(delete(&pool, &uid, &tag.id).await.unwrap());
        assert!(get_one(&pool, &tag.id, &uid).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn merge_same_tag_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let tag = make_tag(&pool, &uid, "T").await;

        assert!(matches!(
            merge(&pool, &uid, &tag.id, &tag.id).await.unwrap_err(),
            DomainError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn merge_reassigns_item_links() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let source = make_tag(&pool, &uid, "Source").await;
        let target = make_tag(&pool, &uid, "Target").await;
        let item_id = make_item(&pool, &uid).await;

        assign_to_item(&pool, &uid, &item_id, &source.id)
            .await
            .unwrap();
        merge(&pool, &uid, &source.id, &target.id).await.unwrap();

        assert!(get_one(&pool, &source.id, &uid).await.unwrap().is_none());
        let tags = get_for_item(&pool, &uid, &item_id).await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].id, target.id);
    }

    #[tokio::test]
    async fn assign_priority_exclusive_enforced() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let item_id = make_item(&pool, &uid).await;

        let p1 = create(
            &pool,
            &uid,
            &CreateTagRequest {
                name: "High".to_string(),
                tag_type: Some("priority".to_string()),
                icon: None,
                color: None,
                parent_tag_id: None,
                metadata: None,
            },
        )
        .await
        .unwrap();
        let p2 = create(
            &pool,
            &uid,
            &CreateTagRequest {
                name: "Low".to_string(),
                tag_type: Some("priority".to_string()),
                icon: None,
                color: None,
                parent_tag_id: None,
                metadata: None,
            },
        )
        .await
        .unwrap();

        assign_to_item(&pool, &uid, &item_id, &p1.id).await.unwrap();
        assert!(matches!(
            assign_to_item(&pool, &uid, &item_id, &p2.id)
                .await
                .unwrap_err(),
            DomainError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn create_duplicate_tag_name_returns_already_exists_with_id() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let first = make_tag(&pool, &uid, "Work").await;
        let err = create(
            &pool,
            &uid,
            &CreateTagRequest {
                name: "work".into(),
                icon: None,
                color: None,
                parent_tag_id: None,
                tag_type: None,
                metadata: None,
            },
        )
        .await
        .unwrap_err();
        match err {
            DomainError::AlreadyExists { kind, id } => {
                assert_eq!(kind, "tag");
                assert_eq!(id, first.id);
            }
            _ => panic!("expected AlreadyExists, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn create_same_name_under_different_parent_ok() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let parent1 = make_tag(&pool, &uid, "Parent1").await;
        let parent2 = make_tag(&pool, &uid, "Parent2").await;
        create(
            &pool,
            &uid,
            &CreateTagRequest {
                name: "Child".into(),
                parent_tag_id: Some(parent1.id.clone()),
                tag_type: None,
                icon: None,
                color: None,
                metadata: None,
            },
        )
        .await
        .unwrap();
        assert!(
            create(
                &pool,
                &uid,
                &CreateTagRequest {
                    name: "Child".into(),
                    parent_tag_id: Some(parent2.id.clone()),
                    tag_type: None,
                    icon: None,
                    color: None,
                    metadata: None,
                },
            )
            .await
            .is_ok()
        );
    }

    #[tokio::test]
    async fn update_tag_name_to_existing_rejected() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let first = make_tag(&pool, &uid, "Work").await;
        let second = make_tag(&pool, &uid, "Personal").await;
        let err = update(
            &pool,
            &uid,
            &second.id,
            &UpdateTagRequest {
                name: Some("work".into()),
                icon: None,
                color: None,
                parent_tag_id: None,
                tag_type: None,
                metadata: None,
            },
        )
        .await
        .unwrap_err();
        match err {
            DomainError::AlreadyExists { id, .. } => assert_eq!(id, first.id),
            _ => panic!("expected AlreadyExists, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn update_same_name_as_self_ok() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let tag = make_tag(&pool, &uid, "Work").await;
        assert!(
            update(
                &pool,
                &uid,
                &tag.id,
                &UpdateTagRequest {
                    name: Some("Work".into()),
                    icon: None,
                    color: None,
                    parent_tag_id: None,
                    tag_type: None,
                    metadata: None,
                },
            )
            .await
            .is_ok()
        );
    }

    // ── filter_home_by_tags tests ─────────────────────────────────────────

    async fn make_list(pool: &SqlitePool, uid: &str) -> String {
        let lid = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES (?, ?, 'L')")
            .bind(&lid)
            .bind(uid)
            .execute(pool)
            .await
            .unwrap();
        lid
    }

    async fn tag_list(pool: &SqlitePool, list_id: &str, tag_id: &str) {
        sqlx::query("INSERT OR IGNORE INTO list_tags (list_id, tag_id) VALUES (?, ?)")
            .bind(list_id)
            .bind(tag_id)
            .execute(pool)
            .await
            .unwrap();
    }

    async fn tag_item(pool: &SqlitePool, item_id: &str, tag_id: &str) {
        sqlx::query("INSERT OR IGNORE INTO item_tags (item_id, tag_id) VALUES (?, ?)")
            .bind(item_id)
            .bind(tag_id)
            .execute(pool)
            .await
            .unwrap();
    }

    async fn make_item_in_list(pool: &SqlitePool, list_id: &str) -> String {
        let iid = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO items (id, list_id, title) VALUES (?, ?, 'I')")
            .bind(&iid)
            .bind(list_id)
            .execute(pool)
            .await
            .unwrap();
        iid
    }

    #[tokio::test]
    async fn test_filter_listwise_single_tag() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let tag_a = make_tag(&pool, &uid, "A").await;
        let tag_b = make_tag(&pool, &uid, "B").await;
        let l1 = make_list(&pool, &uid).await;
        let l2 = make_list(&pool, &uid).await;
        tag_list(&pool, &l1, &tag_a.id).await;
        tag_list(&pool, &l1, &tag_b.id).await;
        tag_list(&pool, &l2, &tag_b.id).await;

        let result = filter_home_by_tags(
            &pool,
            &uid,
            std::slice::from_ref(&tag_a.id),
            kartoteka_shared::FilterMode::Listwise,
        )
        .await
        .unwrap();

        assert_eq!(result.matching_list_ids, vec![l1.clone()]);
        assert!(result.related_tag_ids.contains(&tag_b.id));
        assert!(!result.related_tag_ids.contains(&tag_a.id));
    }

    #[tokio::test]
    async fn test_filter_listwise_multi_tag_and() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let tag_a = make_tag(&pool, &uid, "A").await;
        let tag_b = make_tag(&pool, &uid, "B").await;
        let l1 = make_list(&pool, &uid).await;
        let l2 = make_list(&pool, &uid).await;
        tag_list(&pool, &l1, &tag_a.id).await;
        tag_list(&pool, &l1, &tag_b.id).await;
        tag_list(&pool, &l2, &tag_a.id).await;

        let result = filter_home_by_tags(
            &pool,
            &uid,
            &[tag_a.id.clone(), tag_b.id.clone()],
            kartoteka_shared::FilterMode::Listwise,
        )
        .await
        .unwrap();

        assert_eq!(result.matching_list_ids, vec![l1]);
    }

    #[tokio::test]
    async fn test_filter_itemwise_single_tag() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let tag_a = make_tag(&pool, &uid, "A").await;
        let tag_b = make_tag(&pool, &uid, "B").await;
        let l1 = make_list(&pool, &uid).await;
        let l2 = make_list(&pool, &uid).await;
        let i1 = make_item_in_list(&pool, &l1).await;
        let i2 = make_item_in_list(&pool, &l2).await;
        tag_item(&pool, &i1, &tag_a.id).await;
        tag_item(&pool, &i1, &tag_b.id).await;
        tag_item(&pool, &i2, &tag_b.id).await;

        let result = filter_home_by_tags(
            &pool,
            &uid,
            std::slice::from_ref(&tag_a.id),
            kartoteka_shared::FilterMode::Itemwise,
        )
        .await
        .unwrap();

        assert_eq!(result.matching_list_ids, vec![l1.clone()]);
        assert!(result.related_tag_ids.contains(&tag_b.id));
    }

    #[tokio::test]
    async fn test_filter_itemwise_multi_tag_collective() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let tag_a = make_tag(&pool, &uid, "A").await;
        let tag_b = make_tag(&pool, &uid, "B").await;
        let l1 = make_list(&pool, &uid).await;
        let l2 = make_list(&pool, &uid).await;
        let i1 = make_item_in_list(&pool, &l1).await;
        let i2 = make_item_in_list(&pool, &l1).await;
        let i3 = make_item_in_list(&pool, &l2).await;
        tag_item(&pool, &i1, &tag_a.id).await;
        tag_item(&pool, &i2, &tag_b.id).await;
        tag_item(&pool, &i3, &tag_a.id).await;

        let result = filter_home_by_tags(
            &pool,
            &uid,
            &[tag_a.id.clone(), tag_b.id.clone()],
            kartoteka_shared::FilterMode::Itemwise,
        )
        .await
        .unwrap();

        assert_eq!(result.matching_list_ids, vec![l1]);
    }

    #[tokio::test]
    async fn test_filter_joined_union() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let tag_a = make_tag(&pool, &uid, "A").await;
        let l1 = make_list(&pool, &uid).await;
        let l2 = make_list(&pool, &uid).await;
        tag_list(&pool, &l1, &tag_a.id).await;
        let i1 = make_item_in_list(&pool, &l2).await;
        tag_item(&pool, &i1, &tag_a.id).await;

        let result = filter_home_by_tags(
            &pool,
            &uid,
            std::slice::from_ref(&tag_a.id),
            kartoteka_shared::FilterMode::Joined,
        )
        .await
        .unwrap();

        let mut ids = result.matching_list_ids.clone();
        ids.sort();
        let mut expected = vec![l1, l2];
        expected.sort();
        assert_eq!(ids, expected);
    }
}
