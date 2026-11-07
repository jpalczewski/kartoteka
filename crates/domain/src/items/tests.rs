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
        location_id: None,
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
        location_id: None,
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
        location_id: None,
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
