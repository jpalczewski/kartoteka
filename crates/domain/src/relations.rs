use crate::DomainError;
use kartoteka_db as db;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

// ── Public domain type ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    pub from_type: String,
    pub from_id: String,
    pub to_type: String,
    pub to_id: String,
    pub relation_type: String,
    pub user_id: String,
    pub created_at: String,
    /// `item` / `list` / `container` of the side opposite to the queried entity.
    /// `None` on relations returned by [`create`], which does not resolve titles.
    pub other_type: Option<String>,
    /// Display title of the opposite side: item title, list name or container name.
    pub other_title: Option<String>,
    /// Whether the opposite side is a completed item (only set for items).
    pub other_completed: Option<bool>,
}

fn row_to_relation(row: db::types::EntityRelationRow) -> Relation {
    Relation {
        id: row.id,
        from_type: row.from_type,
        from_id: row.from_id,
        to_type: row.to_type,
        to_id: row.to_id,
        relation_type: row.relation_type,
        user_id: row.user_id,
        created_at: row.created_at,
        other_type: None,
        other_title: None,
        other_completed: None,
    }
}

fn titled_row_to_relation(row: db::relations::EntityRelationWithTitleRow) -> Relation {
    Relation {
        id: row.id,
        from_type: row.from_type,
        from_id: row.from_id,
        to_type: row.to_type,
        to_id: row.to_id,
        relation_type: row.relation_type,
        user_id: row.user_id,
        created_at: row.created_at,
        other_type: Some(row.other_type),
        other_title: row.other_title,
        other_completed: row.other_completed,
    }
}

// ── Entity type validation ────────────────────────────────────────────────────

fn validate_entity_type(entity_type: &str) -> Result<(), DomainError> {
    match entity_type {
        "item" | "list" | "container" => Ok(()),
        _ => Err(DomainError::Validation("invalid_entity_type")),
    }
}

// ── Ownership check ───────────────────────────────────────────────────────────

async fn entity_exists_for_user(
    pool: &SqlitePool,
    user_id: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<bool, DomainError> {
    Ok(match entity_type {
        "item" => db::items::get_one(pool, entity_id, user_id)
            .await?
            .is_some(),
        "list" => db::lists::get_one(pool, entity_id, user_id)
            .await?
            .is_some(),
        "container" => db::containers::get_one(pool, entity_id, user_id)
            .await?
            .is_some(),
        _ => return Err(DomainError::Validation("invalid_entity_type")),
    })
}

// ── Orchestration ─────────────────────────────────────────────────────────────

/// All relations touching an entity, each carrying the display title (and, for
/// items, the completed flag) of the entity on the other side, so the UI never
/// has to render raw ids.
#[tracing::instrument(skip(pool))]
pub async fn get_for_entity(
    pool: &SqlitePool,
    user_id: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<Relation>, DomainError> {
    validate_entity_type(entity_type)?;
    if !entity_exists_for_user(pool, user_id, entity_type, entity_id).await? {
        return Err(DomainError::Forbidden);
    }
    let rows =
        db::relations::list_for_entity_with_titles(pool, entity_type, entity_id, user_id).await?;
    Ok(rows.into_iter().map(titled_row_to_relation).collect())
}

#[tracing::instrument(skip(pool))]
pub async fn create(
    pool: &SqlitePool,
    user_id: &str,
    from_type: &str,
    from_id: &str,
    to_type: &str,
    to_id: &str,
    relation_type: &str,
) -> Result<Relation, DomainError> {
    validate_entity_type(from_type)?;
    validate_entity_type(to_type)?;
    if !entity_exists_for_user(pool, user_id, from_type, from_id).await? {
        return Err(DomainError::Forbidden);
    }
    if !entity_exists_for_user(pool, user_id, to_type, to_id).await? {
        return Err(DomainError::Forbidden);
    }
    let id = Uuid::new_v4().to_string();
    let row = db::relations::insert(
        pool,
        db::relations::InsertRelationInput {
            id: &id,
            from_type,
            from_id,
            to_type,
            to_id,
            relation_type,
            user_id,
        },
    )
    .await?;
    Ok(row_to_relation(row))
}

#[tracing::instrument(skip(pool))]
pub async fn delete(
    pool: &SqlitePool,
    user_id: &str,
    relation_id: &str,
) -> Result<(), DomainError> {
    db::relations::delete(pool, relation_id, user_id).await?;
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{
        create_test_item, create_test_list, create_test_user, test_pool,
    };

    #[tokio::test]
    async fn create_relation_between_owned_items() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_a = create_test_item(&pool, &list_id).await;
        let item_b = create_test_item(&pool, &list_id).await;

        let rel = create(&pool, &uid, "item", &item_a, "item", &item_b, "blocks")
            .await
            .unwrap();
        assert_eq!(rel.from_id, item_a);
        assert_eq!(rel.to_id, item_b);
        assert_eq!(rel.relation_type, "blocks");
    }

    #[tokio::test]
    async fn create_relation_forbidden_for_other_users_entity() {
        let pool = test_pool().await;
        let owner = create_test_user(&pool).await;
        let attacker = create_test_user(&pool).await;
        let owner_list = create_test_list(&pool, &owner).await;
        let attacker_list = create_test_list(&pool, &attacker).await;
        let owner_item = create_test_item(&pool, &owner_list).await;
        let attacker_item = create_test_item(&pool, &attacker_list).await;

        // attacker tries to create a relation targeting owner's item
        let result = create(
            &pool,
            &attacker,
            "item",
            &attacker_item,
            "item",
            &owner_item,
            "blocks",
        )
        .await;
        assert!(matches!(result, Err(DomainError::Forbidden)));
    }

    #[tokio::test]
    async fn get_for_entity_returns_bidirectional() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_a = create_test_item(&pool, &list_id).await;
        let item_b = create_test_item(&pool, &list_id).await;
        let item_c = create_test_item(&pool, &list_id).await;

        create(&pool, &uid, "item", &item_a, "item", &item_b, "blocks")
            .await
            .unwrap();
        create(&pool, &uid, "item", &item_c, "item", &item_a, "relates_to")
            .await
            .unwrap();

        let rels = get_for_entity(&pool, &uid, "item", &item_a).await.unwrap();
        assert_eq!(rels.len(), 2);
    }

    #[tokio::test]
    async fn get_for_entity_includes_other_side_titles_and_completion() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_a = create_test_item(&pool, &list_id).await;
        let item_b = create_test_item(&pool, &list_id).await;

        create(&pool, &uid, "item", &item_b, "item", &item_a, "blocks")
            .await
            .unwrap();
        sqlx::query("UPDATE items SET completed = 1 WHERE id = ?")
            .bind(&item_b)
            .execute(&pool)
            .await
            .unwrap();

        let rels = get_for_entity(&pool, &uid, "item", &item_a).await.unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].other_type.as_deref(), Some("item"));
        assert_eq!(rels[0].other_title.as_deref(), Some("Test Item"));
        assert_eq!(rels[0].other_completed, Some(true));
    }

    #[tokio::test]
    async fn get_for_entity_includes_list_name_title() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_id = create_test_item(&pool, &list_id).await;

        create(
            &pool,
            &uid,
            "item",
            &item_id,
            "list",
            &list_id,
            "relates_to",
        )
        .await
        .unwrap();

        let rels = get_for_entity(&pool, &uid, "item", &item_id).await.unwrap();
        assert_eq!(rels[0].other_type.as_deref(), Some("list"));
        assert_eq!(rels[0].other_title.as_deref(), Some("Test List"));
        assert!(rels[0].other_completed.is_none());

        // From the list side the item is "other".
        let rels = get_for_entity(&pool, &uid, "list", &list_id).await.unwrap();
        assert_eq!(rels[0].other_type.as_deref(), Some("item"));
        assert_eq!(rels[0].other_title.as_deref(), Some("Test Item"));
    }

    #[tokio::test]
    async fn get_for_entity_forbidden_for_other_users_entity() {
        let pool = test_pool().await;
        let owner = create_test_user(&pool).await;
        let attacker = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &owner).await;
        let item = create_test_item(&pool, &list_id).await;

        let result = get_for_entity(&pool, &attacker, "item", &item).await;
        assert!(matches!(result, Err(DomainError::Forbidden)));
    }

    #[tokio::test]
    async fn delete_relation_removes_it() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_test_list(&pool, &uid).await;
        let item_a = create_test_item(&pool, &list_id).await;
        let item_b = create_test_item(&pool, &list_id).await;

        let rel = create(&pool, &uid, "item", &item_a, "item", &item_b, "blocks")
            .await
            .unwrap();
        delete(&pool, &uid, &rel.id).await.unwrap();

        let rels = get_for_entity(&pool, &uid, "item", &item_a).await.unwrap();
        assert!(rels.is_empty());
    }

    #[tokio::test]
    async fn invalid_entity_type_returns_validation_error() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let result = create(&pool, &uid, "bogus", "any", "item", "any", "blocks").await;
        assert!(matches!(result, Err(DomainError::Validation(_))));
    }
}
