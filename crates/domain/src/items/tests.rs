use super::*;
use kartoteka_db::test_helpers::{create_test_list, create_test_tag, create_test_user, test_pool};

async fn create_list(pool: &SqlitePool, user_id: &str, features: &[&str]) -> String {
    let list_id = create_test_list(pool, user_id).await;
    if !features.is_empty() {
        let obj: serde_json::Map<String, serde_json::Value> = features
            .iter()
            .map(|n| (n.to_string(), serde_json::json!({})))
            .collect();
        let json = serde_json::to_string(&obj).unwrap();
        sqlx::query("UPDATE lists SET features = ? WHERE id = ?")
            .bind(json)
            .bind(&list_id)
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
        recurrence_rule: None,
        recurrence_mode: None,
        icon: None,
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
        icon: None,
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
        icon: None,
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
        icon: None,
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
        location_id: None,
        recurrence_rule: None,
        recurrence_mode: None,
        icon: None,
    };

    let updated = update(&pool, &user_id, &item.id, &req)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(updated.item.title, "New");
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
            icon: None,
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
        location_id: None,
        recurrence_rule: None,
        recurrence_mode: None,
        icon: None,
    };

    let updated = update(&pool, &user_id, &item.id, &req)
        .await
        .unwrap()
        .unwrap();

    assert!(updated.item.completed);
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
            icon: None,
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
        location_id: None,
        recurrence_rule: None,
        recurrence_mode: None,
        icon: None,
    };

    let updated = update(&pool, &user_id, &item.id, &req)
        .await
        .unwrap()
        .unwrap();

    assert!(!updated.item.completed);
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
async fn toggle_blocked_when_checklist_feature_missing() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &[]).await;

    let item = create(&pool, &uid, &list_id, &basic_req("A note"))
        .await
        .unwrap();
    let err = toggle_complete(&pool, &uid, &item.id).await.unwrap_err();
    assert!(matches!(err, DomainError::FeatureRequired("checklist")));
}

#[tokio::test]
async fn toggle_allowed_when_checklist_feature_present() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["checklist"]).await;

    let item = create(&pool, &uid, &list_id, &basic_req("Task"))
        .await
        .unwrap();
    assert!(toggle_complete(&pool, &uid, &item.id).await.is_ok());
}

#[tokio::test]
async fn toggle_complete_flips_completed() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let list_id = create_list(&pool, &user_id, &["checklist"]).await;

    let item = create(&pool, &user_id, &list_id, &basic_req("Toggle me"))
        .await
        .unwrap();
    assert!(!item.completed);

    let toggled = toggle_complete(&pool, &user_id, &item.id)
        .await
        .unwrap()
        .unwrap();
    assert!(toggled.item.completed);

    let toggled2 = toggle_complete(&pool, &user_id, &item.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!toggled2.item.completed);
}

#[tokio::test]
async fn toggle_complete_blocked_by_incomplete_item() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let list_id = create_list(&pool, &user_id, &["checklist"]).await;

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
    assert!(item.item.completed);
}

// ── set_status ────────────────────────────────────────────────────────────────

/// An update request that touches nothing but `icon`.
fn icon_req() -> UpdateItemRequest {
    UpdateItemRequest {
        title: None,
        description: None,
        completed: None,
        icon: None,
        quantity: None,
        actual_quantity: None,
        unit: None,
        start_date: None,
        start_time: None,
        deadline: None,
        deadline_time: None,
        hard_deadline: None,
        estimated_duration: None,
        location_id: None,
        recurrence_rule: None,
        recurrence_mode: None,
    }
}

#[tokio::test]
async fn set_status_done_completes_item() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["checklist"]).await;
    let item = create(&pool, &uid, &list_id, &basic_req("Task"))
        .await
        .unwrap();

    let done = set_status(&pool, &uid, &item.id, "done")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(done.status, "done");
    assert!(done.completed);
}

#[tokio::test]
async fn set_status_in_progress_and_todo_clear_completed() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["checklist"]).await;
    let item = create(&pool, &uid, &list_id, &basic_req("Task"))
        .await
        .unwrap();
    set_status(&pool, &uid, &item.id, "done").await.unwrap();

    let in_progress = set_status(&pool, &uid, &item.id, "in_progress")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(in_progress.status, "in_progress");
    assert!(!in_progress.completed);

    let todo = set_status(&pool, &uid, &item.id, "todo")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(todo.status, "todo");
    assert!(!todo.completed);
}

#[tokio::test]
async fn set_status_rejects_unknown_value() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["checklist"]).await;
    let item = create(&pool, &uid, &list_id, &basic_req("Task"))
        .await
        .unwrap();

    let err = set_status(&pool, &uid, &item.id, "blocked")
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        DomainError::Validation("invalid_item_status")
    ));

    let unchanged = get_one(&pool, &item.id, &uid).await.unwrap().unwrap();
    assert_eq!(unchanged.status, "todo");
}

#[tokio::test]
async fn set_status_done_blocked_by_incomplete_item() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["checklist"]).await;
    let blocker = create(&pool, &uid, &list_id, &basic_req("Blocker"))
        .await
        .unwrap();
    let target = create(&pool, &uid, &list_id, &basic_req("Target"))
        .await
        .unwrap();
    let relation_id = uuid::Uuid::new_v4().to_string();
    kartoteka_db::relations::insert(
        &pool,
        kartoteka_db::relations::InsertRelationInput {
            id: &relation_id,
            from_type: "item",
            from_id: &blocker.id,
            to_type: "item",
            to_id: &target.id,
            relation_type: "blocks",
            user_id: &uid,
        },
    )
    .await
    .unwrap();

    let err = set_status(&pool, &uid, &target.id, "done")
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        DomainError::Validation("has_unresolved_blockers")
    ));
    let unchanged = get_one(&pool, &target.id, &uid).await.unwrap().unwrap();
    assert_eq!(unchanged.status, "todo");
    assert!(!unchanged.completed);

    // in_progress is not a completion, so it is allowed
    let in_progress = set_status(&pool, &uid, &target.id, "in_progress")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(in_progress.status, "in_progress");
}

#[tokio::test]
async fn set_status_done_requires_checklist_feature() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &[]).await;
    let item = create(&pool, &uid, &list_id, &basic_req("Note"))
        .await
        .unwrap();

    let err = set_status(&pool, &uid, &item.id, "done").await.unwrap_err();
    assert!(matches!(err, DomainError::FeatureRequired("checklist")));
}

#[tokio::test]
async fn set_status_missing_item_is_none() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    assert!(
        set_status(&pool, &uid, "nonexistent-item", "done")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn create_and_update_icon() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &[]).await;

    let item = create(
        &pool,
        &uid,
        &list_id,
        &CreateItemRequest {
            icon: Some("lucide:leaf".into()),
            ..basic_req("Iconned")
        },
    )
    .await
    .unwrap();
    assert_eq!(item.icon.as_deref(), Some("lucide:leaf"));

    let updated = update(
        &pool,
        &uid,
        &item.id,
        &UpdateItemRequest {
            icon: Some(Some("emoji:🌱".into())),
            ..icon_req()
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(updated.item.icon.as_deref(), Some("emoji:🌱"));

    // Some(None) clears it
    let cleared = update(
        &pool,
        &uid,
        &item.id,
        &UpdateItemRequest {
            icon: Some(None),
            ..icon_req()
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert!(cleared.item.icon.is_none());
}

#[tokio::test]
async fn create_rejects_overlong_icon() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &[]).await;

    let err = create(
        &pool,
        &uid,
        &list_id,
        &CreateItemRequest {
            icon: Some("x".repeat(65)),
            ..basic_req("Too long")
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(err, DomainError::Validation("icon_too_long")));
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
            icon: None,
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
            icon: None,
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
async fn list_all_for_user_returns_items_across_lists() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let list1 = create_list(&pool, &user_id, &[]).await;
    let list2 = create_list(&pool, &user_id, &[]).await;
    super::create(&pool, &user_id, &list1, &basic_req("Item A"))
        .await
        .unwrap();
    super::create(&pool, &user_id, &list2, &basic_req("Item B"))
        .await
        .unwrap();
    let all = super::list_all_for_user(&pool, &user_id).await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn overdue_returns_only_past_incomplete() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let list_id = create_list(&pool, &user_id, &["deadlines", "checklist"]).await;

    // Past deadline, incomplete → should appear
    let past = create(
        &pool,
        &user_id,
        &list_id,
        &CreateItemRequest {
            deadline: Some("2000-01-01".to_string()),
            icon: None,
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
            icon: None,
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
            icon: None,
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

#[tokio::test]
async fn create_item_empty_title_rejected() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let list_id = create_list(&pool, &user_id, &[]).await;

    let err = create(&pool, &user_id, &list_id, &basic_req(""))
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::Validation("title_empty")));
}

#[tokio::test]
async fn create_item_start_after_deadline_rejected() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let list_id = create_list(&pool, &user_id, &["deadlines"]).await;

    let req = CreateItemRequest {
        start_date: Some("2026-05-10".to_string()),
        deadline: Some("2026-05-01".to_string()),
        icon: None,
        ..basic_req("Bad dates")
    };
    let err = create(&pool, &user_id, &list_id, &req).await.unwrap_err();
    assert!(matches!(
        err,
        DomainError::Validation("start_date_after_deadline")
    ));
}

#[tokio::test]
async fn create_item_deadline_after_hard_deadline_rejected() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let list_id = create_list(&pool, &user_id, &["deadlines"]).await;

    let req = CreateItemRequest {
        deadline: Some("2026-05-30".to_string()),
        hard_deadline: Some("2026-05-20".to_string()),
        icon: None,
        ..basic_req("Bad hard deadline")
    };
    let err = create(&pool, &user_id, &list_id, &req).await.unwrap_err();
    assert!(matches!(
        err,
        DomainError::Validation("deadline_after_hard_deadline")
    ));
}

#[tokio::test]
async fn create_item_deadline_time_without_date_rejected() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let list_id = create_list(&pool, &user_id, &["deadlines"]).await;

    let req = CreateItemRequest {
        deadline_time: Some("18:00".to_string()),
        icon: None,
        ..basic_req("Time no date")
    };
    let err = create(&pool, &user_id, &list_id, &req).await.unwrap_err();
    assert!(matches!(
        err,
        DomainError::Validation("deadline_time_without_date")
    ));
}

#[tokio::test]
async fn update_item_empty_title_rejected() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let list_id = create_list(&pool, &user_id, &[]).await;
    let item = create(&pool, &user_id, &list_id, &basic_req("Valid"))
        .await
        .unwrap();

    let req = UpdateItemRequest {
        title: Some("".to_string()),
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
        location_id: None,
        recurrence_rule: None,
        recurrence_mode: None,
        icon: None,
    };
    let err = update(&pool, &user_id, &item.id, &req).await.unwrap_err();
    assert!(matches!(err, DomainError::Validation("title_empty")));
}

#[tokio::test]
async fn update_item_date_order_violation_rejected() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let list_id = create_list(&pool, &user_id, &["deadlines"]).await;
    let item = create(
        &pool,
        &user_id,
        &list_id,
        &CreateItemRequest {
            deadline: Some("2026-05-10".to_string()),
            icon: None,
            ..basic_req("Has deadline")
        },
    )
    .await
    .unwrap();

    // Setting start_date after the existing deadline should fail
    let req = UpdateItemRequest {
        start_date: Some(Some("2026-05-20".to_string())),
        title: None,
        description: None,
        completed: None,
        quantity: None,
        actual_quantity: None,
        unit: None,
        start_time: None,
        deadline: None,
        deadline_time: None,
        hard_deadline: None,
        estimated_duration: None,
        location_id: None,
        recurrence_rule: None,
        recurrence_mode: None,
        icon: None,
    };
    let err = update(&pool, &user_id, &item.id, &req).await.unwrap_err();
    assert!(matches!(
        err,
        DomainError::Validation("start_date_after_deadline")
    ));
}

#[tokio::test]
async fn create_duplicate_title_returns_already_exists_with_id() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &[]).await;
    let first = create(&pool, &uid, &list_id, &basic_req("Buy milk"))
        .await
        .unwrap();
    let err = create(&pool, &uid, &list_id, &basic_req("buy milk"))
        .await
        .unwrap_err();
    match err {
        DomainError::AlreadyExists { kind, id } => {
            assert_eq!(kind, "item");
            assert_eq!(id, first.id);
        }
        _ => panic!("expected AlreadyExists, got {err:?}"),
    }
}

#[tokio::test]
async fn create_same_title_different_list_ok() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list1 = create_list(&pool, &uid, &[]).await;
    let list2 = create_list(&pool, &uid, &[]).await;
    create(&pool, &uid, &list1, &basic_req("Buy milk"))
        .await
        .unwrap();
    assert!(
        create(&pool, &uid, &list2, &basic_req("Buy milk"))
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn update_title_to_existing_rejected() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &[]).await;
    let first = create(&pool, &uid, &list_id, &basic_req("Buy milk"))
        .await
        .unwrap();
    let second = create(&pool, &uid, &list_id, &basic_req("Eggs"))
        .await
        .unwrap();
    let err = update(
        &pool,
        &uid,
        &second.id,
        &UpdateItemRequest {
            title: Some("buy milk".into()),
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
            location_id: None,
            recurrence_rule: None,
            recurrence_mode: None,
            icon: None,
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
async fn get_one_includes_tags_and_comments() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let list_id = create_list(&pool, &user_id, &[]).await;
    let item = create(&pool, &user_id, &list_id, &basic_req("Tagged item"))
        .await
        .unwrap();

    let tag_id = create_test_tag(&pool, &user_id, "Sprint").await;
    sqlx::query("INSERT INTO item_tags (item_id, tag_id) VALUES (?, ?)")
        .bind(&item.id)
        .bind(&tag_id)
        .execute(&pool)
        .await
        .unwrap();

    kartoteka_db::comments::insert(
        &pool,
        kartoteka_db::comments::InsertCommentInput {
            id: &Uuid::new_v4().to_string(),
            entity_type: "item",
            entity_id: &item.id,
            content: "test note",
            author_type: "user",
            author_name: None,
            user_id: &user_id,
        },
    )
    .await
    .unwrap();

    let found = get_one(&pool, &item.id, &user_id).await.unwrap().unwrap();
    assert_eq!(found.tags.len(), 1);
    assert_eq!(found.tags[0].name, "Sprint");
    assert_eq!(found.comments.len(), 1);
    assert_eq!(found.comments[0].content, "test note");
}

#[tokio::test]
async fn list_for_list_includes_tags_and_comments_per_item() {
    let pool = test_pool().await;
    let user_id = create_test_user(&pool).await;
    let list_id = create_list(&pool, &user_id, &[]).await;
    let item_a = create(&pool, &user_id, &list_id, &basic_req("Item A"))
        .await
        .unwrap();
    let item_b = create(&pool, &user_id, &list_id, &basic_req("Item B"))
        .await
        .unwrap();

    let tag_id = create_test_tag(&pool, &user_id, "Sprint").await;
    sqlx::query("INSERT INTO item_tags (item_id, tag_id) VALUES (?, ?)")
        .bind(&item_a.id)
        .bind(&tag_id)
        .execute(&pool)
        .await
        .unwrap();

    kartoteka_db::comments::insert(
        &pool,
        kartoteka_db::comments::InsertCommentInput {
            id: &Uuid::new_v4().to_string(),
            entity_type: "item",
            entity_id: &item_b.id,
            content: "only on B",
            author_type: "user",
            author_name: None,
            user_id: &user_id,
        },
    )
    .await
    .unwrap();

    let items = list_for_list(&pool, &list_id, &user_id).await.unwrap();
    assert_eq!(items.len(), 2);
    let a = items.iter().find(|i| i.id == item_a.id).unwrap();
    let b = items.iter().find(|i| i.id == item_b.id).unwrap();
    assert_eq!(a.tags.len(), 1);
    assert_eq!(a.tags[0].name, "Sprint");
    assert_eq!(a.comments.len(), 0);
    assert_eq!(b.tags.len(), 0);
    assert_eq!(b.comments.len(), 1);
    assert_eq!(b.comments[0].content, "only on B");
}

#[tokio::test]
async fn update_same_title_as_self_ok() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &[]).await;
    let item = create(&pool, &uid, &list_id, &basic_req("Buy milk"))
        .await
        .unwrap();
    assert!(
        update(
            &pool,
            &uid,
            &item.id,
            &UpdateItemRequest {
                title: Some("Buy milk".into()),
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
                location_id: None,
                recurrence_rule: None,
                recurrence_mode: None,
                icon: None,
            },
        )
        .await
        .is_ok()
    );
}

// ── reorder / place (atomic, authorization-checked) ───────────────────────────

/// Position of `id` as stored in the DB, or panic when the row is gone.
async fn stored_position(pool: &SqlitePool, user_id: &str, id: &str) -> (String, i32) {
    let row = db::items::get_one(pool, id, user_id)
        .await
        .unwrap()
        .expect("item row");
    (row.list_id, row.position)
}

#[tokio::test]
async fn reorder_assigns_positions_in_given_order() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &[]).await;
    let a = create(&pool, &uid, &list_id, &basic_req("A"))
        .await
        .unwrap();
    let b = create(&pool, &uid, &list_id, &basic_req("B"))
        .await
        .unwrap();
    let c = create(&pool, &uid, &list_id, &basic_req("C"))
        .await
        .unwrap();

    reorder(
        &pool,
        &uid,
        &list_id,
        &[c.id.clone(), a.id.clone(), b.id.clone()],
    )
    .await
    .unwrap();

    let ordered = db::items::ordered_ids_in_list(&pool, &uid, &list_id)
        .await
        .unwrap();
    assert_eq!(ordered, vec![c.id.clone(), a.id.clone(), b.id.clone()]);
    for (expected, id) in [&c.id, &a.id, &b.id].iter().enumerate() {
        let (list, pos) = stored_position(&pool, &uid, id).await;
        assert_eq!(list, list_id);
        assert_eq!(pos, expected as i32);
    }
}

#[tokio::test]
async fn reorder_with_foreign_id_writes_nothing() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let other_uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &[]).await;
    let other_list_id = create_list(&pool, &other_uid, &[]).await;

    let a = create(&pool, &uid, &list_id, &basic_req("A"))
        .await
        .unwrap();
    let b = create(&pool, &uid, &list_id, &basic_req("B"))
        .await
        .unwrap();
    let foreign = create(&pool, &other_uid, &other_list_id, &basic_req("X"))
        .await
        .unwrap();

    // `a` comes first, so a partial write would already have moved it.
    let result = reorder(
        &pool,
        &uid,
        &list_id,
        &[a.id.clone(), foreign.id.clone(), b.id.clone()],
    )
    .await;
    assert!(matches!(result, Err(DomainError::NotFound("item"))));

    assert_eq!(
        stored_position(&pool, &uid, &a.id).await,
        (list_id.clone(), 0)
    );
    assert_eq!(stored_position(&pool, &uid, &b.id).await, (list_id, 1));
    assert_eq!(
        stored_position(&pool, &other_uid, &foreign.id).await,
        (other_list_id, 0)
    );
}

#[tokio::test]
async fn reorder_rejects_duplicate_ids() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &[]).await;
    let a = create(&pool, &uid, &list_id, &basic_req("A"))
        .await
        .unwrap();

    let result = reorder(&pool, &uid, &list_id, &[a.id.clone(), a.id.clone()]).await;
    assert!(matches!(
        result,
        Err(DomainError::Validation("duplicate_id"))
    ));
}

#[tokio::test]
async fn reorder_foreign_list_is_rejected() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let other_uid = create_test_user(&pool).await;
    let other_list_id = create_list(&pool, &other_uid, &[]).await;

    let result = reorder(&pool, &uid, &other_list_id, &[]).await;
    assert!(matches!(result, Err(DomainError::NotFound("list"))));
}

#[tokio::test]
async fn place_into_foreign_list_fails_and_leaves_item_unchanged() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let other_uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &[]).await;
    let foreign_list_id = create_list(&pool, &other_uid, &[]).await;

    let item = create(&pool, &uid, &list_id, &basic_req("Share me"))
        .await
        .unwrap();

    let result = place(&pool, &uid, &item.id, &foreign_list_id, None).await;
    assert!(matches!(result, Err(DomainError::NotFound("list"))));
    assert_eq!(stored_position(&pool, &uid, &item.id).await, (list_id, 0));
    // Nothing leaked into the foreign list either.
    let foreign_items = db::items::ordered_ids_in_list(&pool, &other_uid, &foreign_list_id)
        .await
        .unwrap();
    assert!(foreign_items.is_empty());
}

#[tokio::test]
async fn place_rejects_duplicate_title_in_target_list() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let source = create_list(&pool, &uid, &[]).await;
    let target = create_list(&pool, &uid, &[]).await;

    let moving = create(&pool, &uid, &source, &basic_req("Milk"))
        .await
        .unwrap();
    create(&pool, &uid, &target, &basic_req("Milk"))
        .await
        .unwrap();

    let result = place(&pool, &uid, &moving.id, &target, None).await;
    assert!(matches!(result, Err(DomainError::AlreadyExists { .. })));
    assert_eq!(stored_position(&pool, &uid, &moving.id).await, (source, 0));
}

#[tokio::test]
async fn place_inserts_before_given_item_and_keeps_positions_dense() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let source = create_list(&pool, &uid, &[]).await;
    let target = create_list(&pool, &uid, &[]).await;

    let moving = create(&pool, &uid, &source, &basic_req("Milk"))
        .await
        .unwrap();
    let first = create(&pool, &uid, &target, &basic_req("Bread"))
        .await
        .unwrap();
    let second = create(&pool, &uid, &target, &basic_req("Eggs"))
        .await
        .unwrap();

    place(&pool, &uid, &moving.id, &target, Some(&second.id))
        .await
        .unwrap();

    let ordered = db::items::ordered_ids_in_list(&pool, &uid, &target)
        .await
        .unwrap();
    assert_eq!(
        ordered,
        vec![first.id.clone(), moving.id.clone(), second.id.clone()]
    );
    for (expected, id) in [&first.id, &moving.id, &second.id].iter().enumerate() {
        assert_eq!(stored_position(&pool, &uid, id).await.1, expected as i32);
    }
    assert!(
        db::items::ordered_ids_in_list(&pool, &uid, &source)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn place_rejects_foreign_before_item() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let other_uid = create_test_user(&pool).await;
    let source = create_list(&pool, &uid, &[]).await;
    let target = create_list(&pool, &uid, &[]).await;
    let foreign_list = create_list(&pool, &other_uid, &[]).await;

    let moving = create(&pool, &uid, &source, &basic_req("Milk"))
        .await
        .unwrap();
    let foreign = create(&pool, &other_uid, &foreign_list, &basic_req("X"))
        .await
        .unwrap();

    let result = place(&pool, &uid, &moving.id, &target, Some(&foreign.id)).await;
    assert!(matches!(result, Err(DomainError::NotFound("item"))));
    assert_eq!(stored_position(&pool, &uid, &moving.id).await, (source, 0));
}

// ── Timezone-aware "today" helper ───────────────────────────────────────────

#[test]
fn date_in_tz_uses_user_timezone_not_utc() {
    // 23:30 UTC is already the next day in Warsaw (UTC+1 in winter).
    let instant = chrono::DateTime::parse_from_rfc3339("2024-01-01T23:30:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    assert_eq!(date_in_tz(instant, "UTC"), "2024-01-01");
    assert_eq!(date_in_tz(instant, "Europe/Warsaw"), "2024-01-02");
}

#[test]
fn date_in_tz_falls_back_to_utc_for_invalid_timezone() {
    let instant = chrono::DateTime::parse_from_rfc3339("2024-06-15T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    assert_eq!(date_in_tz(instant, "Not/AZone"), "2024-06-15");
}

#[tokio::test]
async fn user_today_uses_configured_timezone() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    // The helper always returns a valid YYYY-MM-DD date for the default timezone.
    let today = user_today(&pool, &uid).await.unwrap();
    assert_eq!(today.len(), 10);
    assert_eq!(today, chrono::Utc::now().format("%Y-%m-%d").to_string());
}

// ── Recurrence: spawn the next occurrence on completion ──────────────────────

/// A recurring item: one occurrence per `rule`, anchored at `deadline`.
async fn create_recurring(
    pool: &SqlitePool,
    user_id: &str,
    list_id: &str,
    title: &str,
    deadline: &str,
    rule: &str,
    mode: Option<&str>,
) -> Item {
    create(
        pool,
        user_id,
        list_id,
        &CreateItemRequest {
            deadline: Some(deadline.to_string()),
            recurrence_rule: Some(rule.to_string()),
            recurrence_mode: mode.map(str::to_string),
            ..basic_req(title)
        },
    )
    .await
    .expect("recurring item")
}

fn date_str(date: &Option<FlexDate>) -> Option<String> {
    date.as_ref().map(ToString::to_string)
}

#[tokio::test]
async fn toggle_spawns_next_occurrence() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["checklist", "deadlines", "quantity"]).await;

    let tag_id = create_test_tag(&pool, &uid, "Sprint").await;
    let item = create(
        &pool,
        &uid,
        &list_id,
        &CreateItemRequest {
            description: Some("plants on the balcony".to_string()),
            quantity: Some(3),
            unit: Some("l".to_string()),
            deadline: Some("2026-05-10".to_string()),
            deadline_time: Some("18:00".to_string()),
            estimated_duration: Some(45),
            recurrence_rule: Some("1w".to_string()),
            ..basic_req("Water plants")
        },
    )
    .await
    .unwrap();
    sqlx::query("INSERT INTO item_tags (item_id, tag_id) VALUES (?, ?)")
        .bind(&item.id)
        .bind(&tag_id)
        .execute(&pool)
        .await
        .unwrap();
    kartoteka_db::comments::insert(
        &pool,
        kartoteka_db::comments::InsertCommentInput {
            id: &Uuid::new_v4().to_string(),
            entity_type: "item",
            entity_id: &item.id,
            content: "first time",
            author_type: "user",
            author_name: None,
            user_id: &uid,
        },
    )
    .await
    .unwrap();
    // Another item, so the spawn can only land at the end of the list.
    create(&pool, &uid, &list_id, &basic_req("Later item"))
        .await
        .unwrap();

    let change = toggle_complete(&pool, &uid, &item.id)
        .await
        .unwrap()
        .unwrap();
    let spawned = change.spawned.expect("next occurrence");

    // The rule moves forward: the completed original stops recurring.
    assert!(change.item.completed);
    assert_eq!(change.item.recurrence_rule, None);
    assert_eq!(change.item.recurrence_mode, None);
    assert_eq!(change.item.tags.len(), 1);
    assert_eq!(change.item.comments.len(), 1);

    assert_eq!(spawned.list_id, list_id);
    assert_eq!(spawned.title, "Water plants");
    assert_eq!(
        spawned.description.as_deref(),
        Some("plants on the balcony")
    );
    assert_eq!(date_str(&spawned.deadline).as_deref(), Some("2026-05-17"));
    assert_eq!(spawned.deadline_time.as_deref(), Some("18:00"));
    assert_eq!(spawned.start_date, None);
    assert_eq!(spawned.hard_deadline, None);
    assert_eq!(spawned.quantity, Some(3));
    assert_eq!(spawned.actual_quantity, None);
    assert_eq!(spawned.unit.as_deref(), Some("l"));
    assert_eq!(spawned.estimated_duration, Some(45));
    assert!(!spawned.completed);
    assert_eq!(
        spawned.position, 2,
        "spawned item goes to the end of the list"
    );
    assert_eq!(
        spawned.recurrence_rule,
        Some(RecurrenceRule {
            interval: 1,
            unit: kartoteka_shared::types::RecurrenceUnit::Week
        })
    );
    assert_eq!(spawned.recurrence_mode, Some(RecurrenceMode::Fixed));
    // Tags are copied, comments are not.
    assert_eq!(spawned.tags.len(), 1);
    assert_eq!(spawned.tags[0].name, "Sprint");
    assert!(spawned.comments.is_empty());

    let items = list_for_list(&pool, &list_id, &uid).await.unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items.last().unwrap().id, spawned.id);
}

#[tokio::test]
async fn uncomplete_then_complete_again_spawns_nothing() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["checklist", "deadlines"]).await;
    let item = create_recurring(
        &pool,
        &uid,
        &list_id,
        "Water plants",
        "2026-05-10",
        "1w",
        None,
    )
    .await;

    let first = toggle_complete(&pool, &uid, &item.id)
        .await
        .unwrap()
        .unwrap();
    assert!(first.spawned.is_some());

    let uncompleted = toggle_complete(&pool, &uid, &item.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!uncompleted.item.completed);
    assert!(uncompleted.spawned.is_none());
    // Un-completing does not delete the occurrence that was already spawned.
    assert_eq!(list_for_list(&pool, &list_id, &uid).await.unwrap().len(), 2);

    let again = toggle_complete(&pool, &uid, &item.id)
        .await
        .unwrap()
        .unwrap();
    assert!(again.item.completed);
    assert!(again.spawned.is_none(), "the rule was moved to the spawn");
    assert_eq!(list_for_list(&pool, &list_id, &uid).await.unwrap().len(), 2);
}

#[tokio::test]
async fn update_with_completed_true_spawns_next_occurrence() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["checklist", "deadlines"]).await;
    let item = create_recurring(&pool, &uid, &list_id, "Ping", "2026-05-10", "2d", None).await;

    let change = update(
        &pool,
        &uid,
        &item.id,
        &UpdateItemRequest {
            completed: Some(true),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap();

    assert!(change.item.completed);
    assert_eq!(change.item.recurrence_rule, None);
    let spawned = change.spawned.expect("next occurrence");
    assert_eq!(date_str(&spawned.deadline).as_deref(), Some("2026-05-12"));
    assert!(!spawned.completed);
    assert_eq!(spawned.recurrence_rule, Some("2d".parse().unwrap()));
}

#[tokio::test]
async fn set_status_done_spawns_next_occurrence() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["checklist", "deadlines"]).await;
    let item = create_recurring(&pool, &uid, &list_id, "Ping", "2026-05-10", "2d", None).await;

    let done = set_status(&pool, &uid, &item.id, "done")
        .await
        .unwrap()
        .unwrap();
    assert!(done.completed);
    assert_eq!(done.status, "done");
    assert_eq!(done.recurrence_rule, None);

    let rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT deadline, status, completed FROM items WHERE list_id = ? AND id != ?",
    )
    .bind(&list_id)
    .bind(&item.id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows,
        vec![("2026-05-12".to_string(), "todo".to_string(), 0)]
    );
}

#[tokio::test]
async fn auto_complete_spawns_next_occurrence() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["checklist", "deadlines", "quantity"]).await;
    let item = create(
        &pool,
        &uid,
        &list_id,
        &CreateItemRequest {
            quantity: Some(5),
            deadline: Some("2026-05-10".to_string()),
            recurrence_rule: Some("1m".to_string()),
            ..basic_req("Bottles")
        },
    )
    .await
    .unwrap();

    let change = update(
        &pool,
        &uid,
        &item.id,
        &UpdateItemRequest {
            actual_quantity: Some(Some(5)),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap();

    assert!(change.item.completed);
    assert_eq!(change.item.recurrence_rule, None);
    let spawned = change.spawned.expect("next occurrence");
    assert_eq!(date_str(&spawned.deadline).as_deref(), Some("2026-06-10"));
    assert_eq!(spawned.actual_quantity, None);
}

#[tokio::test]
async fn completing_non_recurring_item_spawns_nothing() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["checklist"]).await;
    let item = create(&pool, &uid, &list_id, &basic_req("One off"))
        .await
        .unwrap();

    let toggled = toggle_complete(&pool, &uid, &item.id)
        .await
        .unwrap()
        .unwrap();
    assert!(toggled.item.completed);
    assert!(toggled.spawned.is_none());

    let other = create(&pool, &uid, &list_id, &basic_req("Another one off"))
        .await
        .unwrap();
    let updated = update(
        &pool,
        &uid,
        &other.id,
        &UpdateItemRequest {
            completed: Some(true),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert!(updated.item.completed);
    assert!(updated.spawned.is_none());
}

#[tokio::test]
async fn unresolved_blocker_aborts_completion_and_spawn() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["checklist", "deadlines"]).await;

    let blocker = create(&pool, &uid, &list_id, &basic_req("Blocker"))
        .await
        .unwrap();
    let target = create_recurring(
        &pool,
        &uid,
        &list_id,
        "Blocked task",
        "2026-05-10",
        "1w",
        None,
    )
    .await;
    kartoteka_db::relations::insert(
        &pool,
        kartoteka_db::relations::InsertRelationInput {
            id: &Uuid::new_v4().to_string(),
            from_type: "item",
            from_id: &blocker.id,
            to_type: "item",
            to_id: &target.id,
            relation_type: "blocks",
            user_id: &uid,
        },
    )
    .await
    .unwrap();

    let err = toggle_complete(&pool, &uid, &target.id).await.unwrap_err();
    assert!(matches!(
        err,
        DomainError::Validation("has_unresolved_blockers")
    ));

    // Nothing was written: no spawn, item still incomplete, rule still there.
    assert_eq!(list_for_list(&pool, &list_id, &uid).await.unwrap().len(), 2);
    let stored = get_one(&pool, &target.id, &uid).await.unwrap().unwrap();
    assert!(!stored.completed);
    assert_eq!(stored.recurrence_rule, Some("1w".parse().unwrap()));
}

#[tokio::test]
async fn floating_rule_anchors_on_user_timezone_today() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["checklist", "deadlines"]).await;

    // Use a timezone that is currently on a different calendar date than UTC,
    // so a UTC anchor and the user's anchor cannot agree.
    let utc_today = chrono::Utc::now().date_naive();
    let (tz_name, local_today) = ["Pacific/Kiritimati", "Pacific/Midway"]
        .iter()
        .map(|name| {
            let tz: chrono_tz::Tz = name.parse().expect("known timezone");
            (*name, chrono::Utc::now().with_timezone(&tz).date_naive())
        })
        .find(|(_, date)| *date != utc_today)
        .expect("one of the extreme timezones is on another date");
    kartoteka_db::preferences::set_timezone(&pool, &uid, tz_name)
        .await
        .unwrap();

    let item = create_recurring(
        &pool,
        &uid,
        &list_id,
        "Water plants",
        "2026-05-10",
        "1w",
        Some("floating"),
    )
    .await;
    let change = toggle_complete(&pool, &uid, &item.id)
        .await
        .unwrap()
        .unwrap();
    let spawned = change.spawned.expect("next occurrence");

    let expected = local_today
        .checked_add_days(chrono::Days::new(7))
        .expect("valid date");
    assert_eq!(
        date_str(&spawned.deadline).as_deref(),
        Some(expected.format("%Y-%m-%d").to_string().as_str())
    );
}

#[tokio::test]
async fn rule_without_date_rejected_on_create() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &[]).await;

    let err = create(
        &pool,
        &uid,
        &list_id,
        &CreateItemRequest {
            recurrence_rule: Some("7d".to_string()),
            ..basic_req("No date")
        },
    )
    .await
    .unwrap_err();

    assert!(matches!(
        err,
        DomainError::Validation("recurrence_requires_date")
    ));
}

#[tokio::test]
async fn clearing_the_only_date_of_a_recurring_item_rejected() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["deadlines"]).await;
    let item = create_recurring(&pool, &uid, &list_id, "Dated", "2026-05-10", "1w", None).await;

    let err = update(
        &pool,
        &uid,
        &item.id,
        &UpdateItemRequest {
            deadline: Some(None),
            ..Default::default()
        },
    )
    .await
    .unwrap_err();

    assert!(matches!(
        err,
        DomainError::Validation("recurrence_requires_date")
    ));
    // Rejected before any write.
    let stored = get_one(&pool, &item.id, &uid).await.unwrap().unwrap();
    assert_eq!(date_str(&stored.deadline).as_deref(), Some("2026-05-10"));
}

#[tokio::test]
async fn invalid_recurrence_strings_rejected() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["deadlines"]).await;

    let bad_rule = create(
        &pool,
        &uid,
        &list_id,
        &CreateItemRequest {
            deadline: Some("2026-05-10".to_string()),
            recurrence_rule: Some("7x".to_string()),
            ..basic_req("Bad rule")
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(
        bad_rule,
        DomainError::Validation("recurrence_invalid_rule")
    ));

    let bad_mode = create(
        &pool,
        &uid,
        &list_id,
        &CreateItemRequest {
            deadline: Some("2026-05-10".to_string()),
            recurrence_rule: Some("7d".to_string()),
            recurrence_mode: Some("sometimes".to_string()),
            ..basic_req("Bad mode")
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(
        bad_mode,
        DomainError::Validation("recurrence_invalid_mode")
    ));

    let mode_without_rule = create(
        &pool,
        &uid,
        &list_id,
        &CreateItemRequest {
            deadline: Some("2026-05-10".to_string()),
            recurrence_mode: Some("floating".to_string()),
            ..basic_req("Dangling mode")
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(
        mode_without_rule,
        DomainError::Validation("recurrence_mode_without_rule")
    ));
}

#[tokio::test]
async fn rule_without_mode_is_stored_as_fixed() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["deadlines"]).await;
    let item = create_recurring(&pool, &uid, &list_id, "Daily", "2026-05-10", "1d", None).await;

    assert_eq!(item.recurrence_rule, Some("1d".parse().unwrap()));
    assert_eq!(item.recurrence_mode, Some(RecurrenceMode::Fixed));
}

#[tokio::test]
async fn clearing_the_rule_clears_the_mode() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["deadlines"]).await;
    let item = create_recurring(
        &pool,
        &uid,
        &list_id,
        "Floating",
        "2026-05-10",
        "1w",
        Some("floating"),
    )
    .await;

    let change = update(
        &pool,
        &uid,
        &item.id,
        &UpdateItemRequest {
            recurrence_rule: Some(None),
            ..Default::default()
        },
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(change.item.recurrence_rule, None);
    assert_eq!(change.item.recurrence_mode, None);
}

#[tokio::test]
async fn duplicate_title_of_completed_item_ok() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["checklist", "deadlines"]).await;

    let done = create(&pool, &uid, &list_id, &basic_req("Eggs"))
        .await
        .unwrap();
    toggle_complete(&pool, &uid, &done.id).await.unwrap();
    // Title uniqueness ignores completed items.
    let again = create(&pool, &uid, &list_id, &basic_req("Eggs"))
        .await
        .unwrap();
    assert_ne!(again.id, done.id);

    // A spawn repeats the title of the item it completes, which by then is
    // completed — that must not be rejected as a duplicate either.
    let other_list = create_list(&pool, &uid, &["checklist", "deadlines"]).await;
    let recurring =
        create_recurring(&pool, &uid, &other_list, "Bread", "2026-05-10", "1w", None).await;
    let change = toggle_complete(&pool, &uid, &recurring.id)
        .await
        .unwrap()
        .unwrap();
    let spawned = change.spawned.expect("next occurrence");
    assert_eq!(spawned.title, "Bread");
    assert_eq!(spawned.title, change.item.title);
}

// ── feature gating on update (#385) ──────────────────────────────────────────

#[tokio::test]
async fn update_setting_deadline_without_feature_is_rejected() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &[]).await;
    let item = create(&pool, &uid, &list_id, &basic_req("No features"))
        .await
        .unwrap();

    let req = UpdateItemRequest {
        deadline: Some(Some("2026-05-15".to_string())),
        ..icon_req()
    };
    let err = update(&pool, &uid, &item.id, &req).await.unwrap_err();
    assert!(matches!(err, DomainError::FeatureRequired("deadlines")));

    // Nothing was written.
    let stored = get_one(&pool, &item.id, &uid).await.unwrap().unwrap();
    assert!(stored.deadline.is_none());
}

#[tokio::test]
async fn update_setting_quantity_without_feature_is_rejected() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &[]).await;
    let item = create(&pool, &uid, &list_id, &basic_req("No features"))
        .await
        .unwrap();

    for req in [
        UpdateItemRequest {
            quantity: Some(Some(5)),
            ..icon_req()
        },
        UpdateItemRequest {
            actual_quantity: Some(Some(2)),
            ..icon_req()
        },
        UpdateItemRequest {
            unit: Some(Some("kg".to_string())),
            ..icon_req()
        },
    ] {
        let err = update(&pool, &uid, &item.id, &req).await.unwrap_err();
        assert!(matches!(err, DomainError::FeatureRequired("quantity")));
    }
}

#[tokio::test]
async fn update_setting_dates_with_features_ok() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["deadlines", "quantity"]).await;
    let item = create(&pool, &uid, &list_id, &basic_req("Has features"))
        .await
        .unwrap();

    let req = UpdateItemRequest {
        deadline: Some(Some("2026-05-15".to_string())),
        quantity: Some(Some(4)),
        ..icon_req()
    };
    let change = update(&pool, &uid, &item.id, &req)
        .await
        .unwrap()
        .expect("item changed");
    assert_eq!(
        change.item.deadline.map(|d| d.to_string()).as_deref(),
        Some("2026-05-15")
    );
    assert_eq!(change.item.quantity, Some(4));
}

#[tokio::test]
async fn update_clearing_dates_without_feature_is_allowed() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["deadlines"]).await;
    let item = create(
        &pool,
        &uid,
        &list_id,
        &CreateItemRequest {
            deadline: Some("2026-05-15".to_string()),
            ..basic_req("Leftover deadline")
        },
    )
    .await
    .unwrap();

    // The feature was turned off after the deadline was stored.
    sqlx::query("UPDATE lists SET features = '{}' WHERE id = ?")
        .bind(&list_id)
        .execute(&pool)
        .await
        .unwrap();

    let req = UpdateItemRequest {
        deadline: Some(None),
        ..icon_req()
    };
    let change = update(&pool, &uid, &item.id, &req)
        .await
        .unwrap()
        .expect("item changed");
    assert!(change.item.deadline.is_none());
}

#[tokio::test]
async fn update_clearing_quantity_without_feature_is_allowed() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, &["quantity"]).await;
    let item = create(
        &pool,
        &uid,
        &list_id,
        &CreateItemRequest {
            quantity: Some(3),
            unit: Some("kg".to_string()),
            ..basic_req("Leftover quantity")
        },
    )
    .await
    .unwrap();

    sqlx::query("UPDATE lists SET features = '{}' WHERE id = ?")
        .bind(&list_id)
        .execute(&pool)
        .await
        .unwrap();

    let req = UpdateItemRequest {
        quantity: Some(None),
        actual_quantity: Some(None),
        unit: Some(None),
        ..icon_req()
    };
    let change = update(&pool, &uid, &item.id, &req)
        .await
        .unwrap()
        .expect("item changed");
    assert!(change.item.quantity.is_none());
    assert!(change.item.unit.is_none());
}
