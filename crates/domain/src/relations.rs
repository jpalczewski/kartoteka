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
    let rows = db::relations::list_for_entity(pool, entity_type, entity_id, user_id).await?;
    Ok(rows.into_iter().map(row_to_relation).collect())
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
    use kartoteka_db::test_helpers::{create_test_user, test_pool};
    use uuid::Uuid;

    async fn insert_test_list(pool: &SqlitePool, user_id: &str) -> String {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO lists (id, user_id, name, list_type) VALUES (?, ?, 'Test List', 'checklist')",
        )
        .bind(&id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("insert_test_list");
        id
    }

    async fn insert_test_item(pool: &SqlitePool, list_id: &str) -> String {
        let id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO items (id, list_id, title) VALUES (?, ?, 'Test Item')")
            .bind(&id)
            .bind(list_id)
            .execute(pool)
            .await
            .expect("insert_test_item");
        id
    }

    #[tokio::test]
    async fn create_relation_between_owned_items() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &uid).await;
        let item_a = insert_test_item(&pool, &list_id).await;
        let item_b = insert_test_item(&pool, &list_id).await;

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
        let owner_list = insert_test_list(&pool, &owner).await;
        let attacker_list = insert_test_list(&pool, &attacker).await;
        let owner_item = insert_test_item(&pool, &owner_list).await;
        let attacker_item = insert_test_item(&pool, &attacker_list).await;

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
        let list_id = insert_test_list(&pool, &uid).await;
        let item_a = insert_test_item(&pool, &list_id).await;
        let item_b = insert_test_item(&pool, &list_id).await;
        let item_c = insert_test_item(&pool, &list_id).await;

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
    async fn get_for_entity_forbidden_for_other_users_entity() {
        let pool = test_pool().await;
        let owner = create_test_user(&pool).await;
        let attacker = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &owner).await;
        let item = insert_test_item(&pool, &list_id).await;

        let result = get_for_entity(&pool, &attacker, "item", &item).await;
        assert!(matches!(result, Err(DomainError::Forbidden)));
    }

    #[tokio::test]
    async fn delete_relation_removes_it() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = insert_test_list(&pool, &uid).await;
        let item_a = insert_test_item(&pool, &list_id).await;
        let item_b = insert_test_item(&pool, &list_id).await;

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
