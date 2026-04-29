use crate::{DomainError, rules};
use kartoteka_db::{
    self as db,
    tags::{InsertTagInput, UpdateTagInput},
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
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

// ── Public functions ──────────────────────────────────────────────────────────

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

#[tracing::instrument(skip(pool))]
pub async fn create(
    pool: &SqlitePool,
    user_id: &str,
    req: &CreateTagRequest,
) -> Result<Tag, DomainError> {
    // Phase 1: READ — fetch parent type for hierarchy validation
    let parent_type: Option<String> = if let Some(ref parent_id) = req.parent_tag_id {
        let parent = db::tags::get_one(pool, parent_id, user_id)
            .await?
            .ok_or(DomainError::NotFound("parent_tag"))?;
        Some(parent.tag_type)
    } else {
        None
    };

    // Phase 2: THINK
    let tag_type = req.tag_type.as_deref().unwrap_or("tag");
    rules::tags::validate_location_hierarchy(tag_type, parent_type.as_deref())?;

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

    // Phase 2: THINK — validate parent change and hierarchy
    let effective_type = req.tag_type.as_deref().unwrap_or(&current.tag_type);

    match &req.parent_tag_id {
        Some(Some(new_parent_id)) => {
            // Cycle detection
            let ancestors = db::tags::get_ancestors(pool, new_parent_id, user_id).await?;
            rules::tags::validate_parent(id, new_parent_id, &ancestors)?;
            // Location hierarchy with new parent
            let parent = db::tags::get_one(pool, new_parent_id, user_id)
                .await?
                .ok_or(DomainError::NotFound("parent_tag"))?;
            rules::tags::validate_location_hierarchy(effective_type, Some(&parent.tag_type))?;
        }
        Some(None) => {
            // Clearing parent — validate hierarchy with no parent
            rules::tags::validate_location_hierarchy(effective_type, None)?;
        }
        None => {
            // Parent unchanged; only re-validate if tag_type is changing
            if req.tag_type.is_some() {
                let parent_type = match &current.parent_tag_id {
                    Some(pid) => db::tags::get_one(pool, pid, user_id)
                        .await?
                        .map(|p| p.tag_type),
                    None => None,
                };
                rules::tags::validate_location_hierarchy(effective_type, parent_type.as_deref())?;
            }
        }
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
    db::tags::add_item_tag(pool, item_id, tag_id, user_id).await?;
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
    db::tags::add_list_tag(pool, list_id, tag_id, user_id).await?;
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
    db::tags::add_container_tag(pool, container_id, tag_id, user_id).await?;
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

#[derive(Serialize)]
pub struct TagEntities {
    pub items: Vec<db::tags::TaggedItemRow>,
    pub lists: Vec<db::tags::TaggedListRow>,
}

/// Returns items and lists linked to `tag_id`.
/// `entity_type`: `Some("item")`, `Some("list")`, or `None` for both.
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

    let items = match entity_type {
        Some(t) if t != "item" => vec![],
        _ => db::tags::get_items_by_tag(pool, tag_id, user_id).await?,
    };
    let lists = match entity_type {
        Some(t) if t != "list" => vec![],
        _ => db::tags::get_lists_by_tag(pool, tag_id, user_id).await?,
    };

    Ok(TagEntities { items, lists })
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
    async fn city_tag_requires_country_parent() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let generic = make_tag(&pool, &uid, "Generic").await;

        let result = create(
            &pool,
            &uid,
            &CreateTagRequest {
                name: "Warsaw".to_string(),
                tag_type: Some("city".to_string()),
                parent_tag_id: Some(generic.id.clone()),
                icon: None,
                color: None,
                metadata: None,
            },
        )
        .await;

        assert!(matches!(result.unwrap_err(), DomainError::Validation(_)));
    }

    #[tokio::test]
    async fn city_with_country_parent_ok() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let country = create(
            &pool,
            &uid,
            &CreateTagRequest {
                name: "Poland".to_string(),
                tag_type: Some("country".to_string()),
                icon: None,
                color: None,
                parent_tag_id: None,
                metadata: None,
            },
        )
        .await
        .unwrap();

        let result = create(
            &pool,
            &uid,
            &CreateTagRequest {
                name: "Warsaw".to_string(),
                tag_type: Some("city".to_string()),
                parent_tag_id: Some(country.id.clone()),
                icon: None,
                color: None,
                metadata: None,
            },
        )
        .await;

        assert!(result.is_ok());
    }
}
