use crate::{rules, DomainError};
use kartoteka_db as db;
use kartoteka_shared::types::FlexDate;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

// ── Public domain types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub description: Option<String>,
    pub completed: bool,
    pub position: i32,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<FlexDate>,
    pub start_time: Option<String>,
    pub deadline: Option<FlexDate>,
    pub deadline_time: Option<String>,
    pub hard_deadline: Option<FlexDate>,
    pub estimated_duration: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateItemRequest {
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

/// All fields optional. None = don't update. For nullable: Some(None) = clear to NULL.
#[derive(Debug, Deserialize)]
pub struct UpdateItemRequest {
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

#[derive(Debug, Deserialize)]
pub struct MoveItemRequest {
    pub position: i32,
    pub list_id: Option<String>, // move to different list
}

// ── Conversion from db row ────────────────────────────────────────────────────

fn row_to_item(row: db::types::ItemRow) -> Item {
    Item {
        id: row.id,
        list_id: row.list_id,
        title: row.title,
        description: row.description,
        completed: row.completed,
        position: row.position,
        quantity: row.quantity,
        actual_quantity: row.actual_quantity,
        unit: row.unit,
        start_date: row.start_date,
        start_time: row.start_time,
        deadline: row.deadline,
        deadline_time: row.deadline_time,
        hard_deadline: row.hard_deadline,
        estimated_duration: row.estimated_duration,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

// ── Orchestration ─────────────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn list_for_list(
    pool: &SqlitePool,
    list_id: &str,
    user_id: &str,
) -> Result<Vec<Item>, DomainError> {
    let rows = db::items::list_for_list(pool, list_id, user_id).await?;
    Ok(rows.into_iter().map(row_to_item).collect())
}

#[tracing::instrument(skip(pool))]
pub async fn get_one(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<Item>, DomainError> {
    Ok(db::items::get_one(pool, id, user_id).await?.map(row_to_item))
}

#[tracing::instrument(skip(pool))]
pub async fn create(
    pool: &SqlitePool,
    user_id: &str,
    list_id: &str,
    req: &CreateItemRequest,
) -> Result<Item, DomainError> {
    // Phase 1: READ
    let ctx = db::lists::get_create_item_context(pool, list_id, user_id)
        .await?
        .ok_or(DomainError::NotFound("list"))?;

    // Phase 2: THINK
    let has_dates =
        req.start_date.is_some() || req.deadline.is_some() || req.hard_deadline.is_some();
    let has_quantity = req.quantity.is_some() || req.unit.is_some();
    rules::items::validate_features(&ctx.features, has_dates, has_quantity)?;

    // Phase 3: WRITE
    let row = db::items::insert(
        pool,
        &db::items::InsertItemInput {
            id: Uuid::new_v4().to_string(),
            list_id: list_id.to_string(),
            position: i32::try_from(ctx.next_position).unwrap_or(i32::MAX),
            title: req.title.clone(),
            description: req.description.clone(),
            quantity: req.quantity,
            actual_quantity: req.actual_quantity,
            unit: req.unit.clone(),
            start_date: req.start_date.clone(),
            start_time: req.start_time.clone(),
            deadline: req.deadline.clone(),
            deadline_time: req.deadline_time.clone(),
            hard_deadline: req.hard_deadline.clone(),
            estimated_duration: req.estimated_duration,
        },
    )
    .await?;

    Ok(row_to_item(row))
}

// ── Integration tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{create_test_user, test_pool};

    async fn create_list(pool: &SqlitePool, user_id: &str, features: &[&str]) -> String {
        let list_id = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES (?, ?, 'Test List')")
            .bind(&list_id)
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
        for feature in features {
            sqlx::query(
                "INSERT INTO list_features (list_id, feature_name) VALUES (?, ?)",
            )
            .bind(&list_id)
            .bind(feature)
            .execute(pool)
            .await
            .unwrap();
        }
        list_id
    }

    fn basic_req(title: &str) -> CreateItemRequest {
        CreateItemRequest {
            title: title.to_string(),
            description: None,
            quantity: None,
            actual_quantity: None,
            unit: None,
            start_date: None,
            start_time: None,
            deadline: None,
            deadline_time: None,
            hard_deadline: None,
            estimated_duration: None,
        }
    }

    #[tokio::test]
    async fn create_item_basic() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_list(&pool, &user_id, &[]).await;

        let item = create(&pool, &user_id, &list_id, &basic_req("Buy milk"))
            .await
            .unwrap();

        assert_eq!(item.title, "Buy milk");
        assert_eq!(item.list_id, list_id);
        assert!(!item.completed);
        assert_eq!(item.position, 0);
    }

    #[tokio::test]
    async fn create_item_position_increments() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_list(&pool, &user_id, &[]).await;

        let i1 = create(&pool, &user_id, &list_id, &basic_req("First"))
            .await
            .unwrap();
        let i2 = create(&pool, &user_id, &list_id, &basic_req("Second"))
            .await
            .unwrap();
        let i3 = create(&pool, &user_id, &list_id, &basic_req("Third"))
            .await
            .unwrap();

        assert_eq!(i1.position, 0);
        assert_eq!(i2.position, 1);
        assert_eq!(i3.position, 2);
    }

    #[tokio::test]
    async fn create_item_unknown_list_rejected() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;

        let err = create(&pool, &user_id, "nonexistent-list", &basic_req("Item"))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound("list")));
    }

    #[tokio::test]
    async fn create_item_dates_without_deadlines_feature_rejected() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_list(&pool, &user_id, &[]).await;

        let req = CreateItemRequest {
            deadline: Some("2026-05-15".to_string()),
            ..basic_req("Deadline item")
        };

        let err = create(&pool, &user_id, &list_id, &req)
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::FeatureRequired("deadlines")));
    }

    #[tokio::test]
    async fn create_item_dates_with_deadlines_feature_ok() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_list(&pool, &user_id, &["deadlines"]).await;

        let req = CreateItemRequest {
            deadline: Some("2026-05-15".to_string()),
            ..basic_req("Deadline item")
        };

        let item = create(&pool, &user_id, &list_id, &req).await.unwrap();

        assert!(item.deadline.is_some());
    }

    #[tokio::test]
    async fn create_item_quantity_without_feature_rejected() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_list(&pool, &user_id, &[]).await;

        let req = CreateItemRequest {
            quantity: Some(3),
            ..basic_req("Qty item")
        };

        let err = create(&pool, &user_id, &list_id, &req)
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::FeatureRequired("quantity")));
    }

    #[tokio::test]
    async fn get_one_returns_item() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_list(&pool, &user_id, &[]).await;

        let created = create(&pool, &user_id, &list_id, &basic_req("Find me"))
            .await
            .unwrap();

        let found = get_one(&pool, &created.id, &user_id).await.unwrap();

        assert!(found.is_some());
        let item = found.unwrap();
        assert_eq!(item.id, created.id);
        assert_eq!(item.title, "Find me");
    }

    #[tokio::test]
    async fn list_for_list_returns_all_items() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_list(&pool, &user_id, &[]).await;

        create(&pool, &user_id, &list_id, &basic_req("Item A"))
            .await
            .unwrap();
        create(&pool, &user_id, &list_id, &basic_req("Item B"))
            .await
            .unwrap();

        let items = list_for_list(&pool, &list_id, &user_id).await.unwrap();

        assert_eq!(items.len(), 2);
    }
}
