use crate::{DomainError, rules};
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
    Ok(db::items::get_one(pool, id, user_id)
        .await?
        .map(row_to_item))
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

#[tracing::instrument(skip(pool))]
pub async fn update(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
    req: &UpdateItemRequest,
) -> Result<Option<Item>, DomainError> {
    let input = db::items::UpdateItemInput {
        title: req.title.clone(),
        description: req.description.clone(),
        completed: req.completed,
        quantity: req.quantity,
        actual_quantity: req.actual_quantity,
        unit: req.unit.clone(),
        start_date: req.start_date.clone(),
        start_time: req.start_time.clone(),
        deadline: req.deadline.clone(),
        deadline_time: req.deadline_time.clone(),
        hard_deadline: req.hard_deadline.clone(),
        estimated_duration: req.estimated_duration,
    };
    let found = db::items::update(pool, id, user_id, &input).await?;
    if !found {
        return Ok(None);
    }
    let item = match db::items::get_one(pool, id, user_id)
        .await?
        .map(row_to_item)
    {
        Some(i) => i,
        None => return Ok(None),
    };
    // Auto-complete check
    if !item.completed {
        if let (Some(actual), Some(qty)) = (item.actual_quantity, item.quantity) {
            if rules::items::should_auto_complete(actual, qty) {
                db::items::set_completed(pool, id, true).await?;
                return Ok(db::items::get_one(pool, id, user_id)
                    .await?
                    .map(row_to_item));
            }
        }
    }
    Ok(Some(item))
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, user_id: &str, id: &str) -> Result<bool, DomainError> {
    Ok(db::items::delete(pool, id, user_id).await?)
}

#[tracing::instrument(skip(pool))]
pub async fn toggle_complete(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
) -> Result<Option<Item>, DomainError> {
    // Phase 1 READ: check current state
    let item = match db::items::get_one(pool, id, user_id).await? {
        Some(row) => row_to_item(row),
        None => return Ok(None),
    };

    // Phase 2 THINK: only check blockers when completing (false → true)
    if !item.completed {
        let blockers = db::relations::get_unresolved_blockers(pool, id).await?;
        crate::rules::items::validate_can_complete(blockers.len())?;
    }

    // Phase 3 WRITE: toggle
    let found = db::items::toggle_complete(pool, id, user_id).await?;
    if !found {
        return Ok(None);
    }
    Ok(db::items::get_one(pool, id, user_id)
        .await?
        .map(row_to_item))
}

#[tracing::instrument(skip(pool))]
pub async fn move_item(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
    req: &MoveItemRequest,
) -> Result<Option<Item>, DomainError> {
    // Phase 1 READ: validate target list if provided
    if let Some(target_list_id) = &req.list_id {
        if db::lists::get_one(pool, target_list_id, user_id)
            .await?
            .is_none()
        {
            return Err(DomainError::NotFound("list"));
        }
    }
    // Phase 3 WRITE
    let found =
        db::items::move_item(pool, id, user_id, req.position, req.list_id.as_deref()).await?;
    if !found {
        return Ok(None);
    }
    Ok(db::items::get_one(pool, id, user_id)
        .await?
        .map(row_to_item))
}

#[tracing::instrument(skip(pool))]
pub async fn by_date(
    pool: &SqlitePool,
    user_id: &str,
    date: &str,
) -> Result<Vec<Item>, DomainError> {
    let resolved = if date == "today" {
        let tz_str = db::preferences::get_timezone(pool, user_id).await?;
        let tz: chrono_tz::Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
        chrono::Utc::now()
            .with_timezone(&tz)
            .date_naive()
            .format("%Y-%m-%d")
            .to_string()
    } else {
        date.to_string()
    };
    let rows = db::items::by_date(pool, user_id, &resolved).await?;
    Ok(rows.into_iter().map(row_to_item).collect())
}

#[tracing::instrument(skip(pool))]
pub async fn calendar(
    pool: &SqlitePool,
    user_id: &str,
    year_month: &str,
) -> Result<Vec<Item>, DomainError> {
    let rows = db::items::calendar(pool, user_id, year_month).await?;
    Ok(rows.into_iter().map(row_to_item).collect())
}

/// Returns incomplete items with `deadline` strictly before today in the user's timezone.
/// Hard deadlines are intentionally excluded — overdue means a missed `deadline` date.
#[tracing::instrument(skip(pool))]
pub async fn overdue(pool: &SqlitePool, user_id: &str) -> Result<Vec<Item>, DomainError> {
    let tz_str = db::preferences::get_timezone(pool, user_id).await?;
    let tz: chrono_tz::Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
    let today = chrono::Utc::now()
        .with_timezone(&tz)
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let rows = db::items::overdue(pool, user_id, &today).await?;
    Ok(rows.into_iter().map(row_to_item).collect())
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
            sqlx::query("INSERT INTO list_features (list_id, feature_name) VALUES (?, ?)")
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

        let err = create(&pool, &user_id, &list_id, &req).await.unwrap_err();

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

        let err = create(&pool, &user_id, &list_id, &req).await.unwrap_err();

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

    #[tokio::test]
    async fn update_title() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_list(&pool, &user_id, &[]).await;

        let item = create(&pool, &user_id, &list_id, &basic_req("Old title"))
            .await
            .unwrap();

        let req = UpdateItemRequest {
            title: Some("New".to_string()),
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
        };

        let updated = update(&pool, &user_id, &item.id, &req)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(updated.title, "New");
    }

    #[tokio::test]
    async fn update_triggers_auto_complete() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_list(&pool, &user_id, &["quantity"]).await;

        let item = create(
            &pool,
            &user_id,
            &list_id,
            &CreateItemRequest {
                quantity: Some(5),
                ..basic_req("Qty item")
            },
        )
        .await
        .unwrap();

        let req = UpdateItemRequest {
            actual_quantity: Some(Some(5)),
            title: None,
            description: None,
            completed: None,
            quantity: None,
            unit: None,
            start_date: None,
            start_time: None,
            deadline: None,
            deadline_time: None,
            hard_deadline: None,
            estimated_duration: None,
        };

        let updated = update(&pool, &user_id, &item.id, &req)
            .await
            .unwrap()
            .unwrap();

        assert!(updated.completed);
    }

    #[tokio::test]
    async fn update_does_not_auto_complete_when_below_target() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_list(&pool, &user_id, &["quantity"]).await;

        let item = create(
            &pool,
            &user_id,
            &list_id,
            &CreateItemRequest {
                quantity: Some(5),
                ..basic_req("Qty item")
            },
        )
        .await
        .unwrap();

        let req = UpdateItemRequest {
            actual_quantity: Some(Some(4)),
            title: None,
            description: None,
            completed: None,
            quantity: None,
            unit: None,
            start_date: None,
            start_time: None,
            deadline: None,
            deadline_time: None,
            hard_deadline: None,
            estimated_duration: None,
        };

        let updated = update(&pool, &user_id, &item.id, &req)
            .await
            .unwrap()
            .unwrap();

        assert!(!updated.completed);
    }

    #[tokio::test]
    async fn delete_item() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_list(&pool, &user_id, &[]).await;

        let item = create(&pool, &user_id, &list_id, &basic_req("Doomed"))
            .await
            .unwrap();

        let deleted = delete(&pool, &user_id, &item.id).await.unwrap();
        assert!(deleted);

        let found = get_one(&pool, &item.id, &user_id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn toggle_complete_flips_completed() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_list(&pool, &user_id, &[]).await;

        let item = create(&pool, &user_id, &list_id, &basic_req("Toggle me"))
            .await
            .unwrap();
        assert!(!item.completed);

        let toggled = toggle_complete(&pool, &user_id, &item.id)
            .await
            .unwrap()
            .unwrap();
        assert!(toggled.completed);

        let toggled2 = toggle_complete(&pool, &user_id, &item.id)
            .await
            .unwrap()
            .unwrap();
        assert!(!toggled2.completed);
    }

    #[tokio::test]
    async fn toggle_complete_blocked_by_incomplete_item() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_list(&pool, &user_id, &[]).await;

        let blocker = create(&pool, &user_id, &list_id, &basic_req("Blocker"))
            .await
            .unwrap();

        let target = create(&pool, &user_id, &list_id, &basic_req("Target"))
            .await
            .unwrap();

        // Create blocking relation
        kartoteka_db::relations::insert(
            &pool,
            kartoteka_db::relations::InsertRelationInput {
                id: &uuid::Uuid::new_v4().to_string(),
                from_type: "item",
                from_id: &blocker.id,
                to_type: "item",
                to_id: &target.id,
                relation_type: "blocks",
                user_id: &user_id,
            },
        )
        .await
        .unwrap();

        // Target cannot be completed while blocker is incomplete
        let result = toggle_complete(&pool, &user_id, &target.id).await;
        assert!(matches!(
            result,
            Err(DomainError::Validation("has_unresolved_blockers"))
        ));

        // Complete the blocker
        toggle_complete(&pool, &user_id, &blocker.id).await.unwrap();

        // Now target can be completed
        let item = toggle_complete(&pool, &user_id, &target.id)
            .await
            .unwrap()
            .unwrap();
        assert!(item.completed);
    }

    #[tokio::test]
    async fn move_item_changes_position() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_list(&pool, &user_id, &[]).await;

        let item = create(&pool, &user_id, &list_id, &basic_req("Move me"))
            .await
            .unwrap();
        assert_eq!(item.position, 0);

        let req = MoveItemRequest {
            position: 3,
            list_id: None,
        };
        let moved = move_item(&pool, &user_id, &item.id, &req)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(moved.position, 3);
    }

    #[tokio::test]
    async fn by_date_returns_items_on_date() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_list(&pool, &user_id, &["deadlines"]).await;

        create(
            &pool,
            &user_id,
            &list_id,
            &CreateItemRequest {
                deadline: Some("2026-06-01".to_string()),
                ..basic_req("June item")
            },
        )
        .await
        .unwrap();

        let items = by_date(&pool, &user_id, "2026-06-01").await.unwrap();
        assert_eq!(items.len(), 1);

        let items_empty = by_date(&pool, &user_id, "2026-06-02").await.unwrap();
        assert_eq!(items_empty.len(), 0);
    }

    #[tokio::test]
    async fn calendar_returns_items_in_month() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_list(&pool, &user_id, &["deadlines"]).await;

        create(
            &pool,
            &user_id,
            &list_id,
            &CreateItemRequest {
                deadline: Some("2026-07-15".to_string()),
                ..basic_req("July item")
            },
        )
        .await
        .unwrap();

        let items = calendar(&pool, &user_id, "2026-07").await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "July item");

        let items_empty = calendar(&pool, &user_id, "2026-08").await.unwrap();
        assert_eq!(items_empty.len(), 0);
    }

    #[tokio::test]
    async fn overdue_returns_only_past_incomplete() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        let list_id = create_list(&pool, &user_id, &["deadlines"]).await;

        // Past deadline, incomplete → should appear
        let past = create(
            &pool,
            &user_id,
            &list_id,
            &CreateItemRequest {
                deadline: Some("2000-01-01".to_string()),
                ..basic_req("Past task")
            },
        )
        .await
        .unwrap();

        // Future deadline, incomplete → should NOT appear
        create(
            &pool,
            &user_id,
            &list_id,
            &CreateItemRequest {
                deadline: Some("9999-12-31".to_string()),
                ..basic_req("Future task")
            },
        )
        .await
        .unwrap();

        // Past deadline but completed → should NOT appear
        let completed = create(
            &pool,
            &user_id,
            &list_id,
            &CreateItemRequest {
                deadline: Some("2000-01-01".to_string()),
                ..basic_req("Done overdue")
            },
        )
        .await
        .unwrap();
        toggle_complete(&pool, &user_id, &completed.id)
            .await
            .unwrap();

        let result = overdue(&pool, &user_id).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, past.id);
    }
}
