//! Database-backed tests for calendar tokens, reminder defaults and feed assembly.

use super::*;
use crate::DomainError;
use kartoteka_db::test_helpers::{create_test_user, test_pool};
use kartoteka_shared::ReminderDefaults;
use sqlx::SqlitePool;
use uuid::Uuid;

/// Far future, so items are always inside the feed's "last 30 days" window.
const FUTURE: &str = "2999-01-01";

async fn create_list(pool: &SqlitePool, user_id: &str, name: &str) -> String {
    let list_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO lists (id, user_id, name) VALUES (?, ?, ?)")
        .bind(&list_id)
        .bind(user_id)
        .bind(name)
        .execute(pool)
        .await
        .unwrap();
    list_id
}

async fn create_sublist(pool: &SqlitePool, user_id: &str, name: &str, parent_id: &str) -> String {
    let list_id = create_list(pool, user_id, name).await;
    sqlx::query("UPDATE lists SET parent_list_id = ? WHERE id = ?")
        .bind(parent_id)
        .bind(&list_id)
        .execute(pool)
        .await
        .unwrap();
    list_id
}

async fn create_item(pool: &SqlitePool, list_id: &str, title: &str) -> String {
    let item_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO items (id, list_id, title, start_date) VALUES (?, ?, ?, ?) \
         RETURNING id",
    )
    .bind(&item_id)
    .bind(list_id)
    .bind(title)
    .bind(FUTURE)
    .execute(pool)
    .await
    .unwrap();
    item_id
}

#[tokio::test]
async fn created_token_is_43_chars_and_produces_a_feed() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, "Groceries").await;
    create_item(&pool, &list_id, "Buy milk").await;

    let created = create_token(&pool, &uid, "My feed", None).await.unwrap();
    assert_eq!(created.token.len(), 43);
    assert_eq!(created.name, "My feed");
    assert!(created.list_id.is_none());

    let feed = feed_for_token(&pool, &created.token)
        .await
        .unwrap()
        .unwrap();
    assert!(feed.body.contains("Buy milk"), "body:\n{}", feed.body);
    assert!(feed.body.contains("X-WR-CALNAME:Kartoteka"));
    assert!(feed.etag.starts_with('"') && feed.etag.ends_with('"'));
}

#[tokio::test]
async fn revoked_token_returns_none() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let created = create_token(&pool, &uid, "Feed", None).await.unwrap();
    assert!(
        feed_for_token(&pool, &created.token)
            .await
            .unwrap()
            .is_some()
    );

    revoke_token(&pool, &uid, &created.id).await.unwrap();
    assert!(
        feed_for_token(&pool, &created.token)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn revoke_foreign_token_returns_not_found() {
    let pool = test_pool().await;
    let owner = create_test_user(&pool).await;
    let other = create_test_user(&pool).await;
    let created = create_token(&pool, &owner, "Feed", None).await.unwrap();

    let err = revoke_token(&pool, &other, &created.id).await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound("calendar_token")));
    assert!(
        feed_for_token(&pool, &created.token)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn malformed_token_returns_none() {
    let pool = test_pool().await;
    assert!(feed_for_token(&pool, "").await.unwrap().is_none());
    assert!(feed_for_token(&pool, "too-short").await.unwrap().is_none());
    // 43 characters, but not from the base64url alphabet.
    let illegal = "!".repeat(43);
    assert!(feed_for_token(&pool, &illegal).await.unwrap().is_none());
    // 44 characters is one too many.
    let long = "a".repeat(44);
    assert!(feed_for_token(&pool, &long).await.unwrap().is_none());
}

#[tokio::test]
async fn random_well_formed_token_returns_none() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    create_token(&pool, &uid, "Feed", None).await.unwrap();

    let random = "A".repeat(43);
    assert!(feed_for_token(&pool, &random).await.unwrap().is_none());
}

#[tokio::test]
async fn list_scoped_token_covers_list_and_direct_sublists_only() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let parent = create_list(&pool, &uid, "Parent").await;
    let child = create_sublist(&pool, &uid, "Child", &parent).await;
    let grandchild = create_sublist(&pool, &uid, "Grandchild", &child).await;
    let unrelated = create_list(&pool, &uid, "Unrelated").await;

    create_item(&pool, &parent, "In parent").await;
    create_item(&pool, &child, "In child").await;
    create_item(&pool, &grandchild, "In grandchild").await;
    create_item(&pool, &unrelated, "In unrelated").await;

    let created = create_token(&pool, &uid, "Parent feed", Some(&parent))
        .await
        .unwrap();
    assert_eq!(created.list_id.as_deref(), Some(parent.as_str()));

    let feed = feed_for_token(&pool, &created.token)
        .await
        .unwrap()
        .unwrap();
    assert!(feed.body.contains("X-WR-CALNAME:Parent"));
    assert!(feed.body.contains("In parent"));
    assert!(feed.body.contains("In child"));
    assert!(!feed.body.contains("In grandchild"));
    assert!(!feed.body.contains("In unrelated"));
}

#[tokio::test]
async fn foreign_list_returns_not_found() {
    let pool = test_pool().await;
    let owner = create_test_user(&pool).await;
    let other = create_test_user(&pool).await;
    let list_id = create_list(&pool, &owner, "Owner list").await;

    let err = create_token(&pool, &other, "Feed", Some(&list_id))
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::NotFound("list")));

    let missing = Uuid::new_v4().to_string();
    let err = create_token(&pool, &other, "Feed", Some(&missing))
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::NotFound("list")));
}

#[tokio::test]
async fn empty_name_is_rejected() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    for name in ["", "   ", "\t\n"] {
        let err = create_token(&pool, &uid, name, None).await.unwrap_err();
        assert!(
            matches!(err, DomainError::Validation("calendar_token_name_empty")),
            "unexpected error for {name:?}: {err:?}"
        );
    }
}

#[tokio::test]
async fn token_name_is_trimmed_and_length_limited() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;

    let created = create_token(&pool, &uid, "  Trimmed  ", None)
        .await
        .unwrap();
    assert_eq!(created.name, "Trimmed");

    let too_long = "x".repeat(101);
    let err = create_token(&pool, &uid, &too_long, None)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        DomainError::Validation("calendar_token_name_too_long")
    ));

    let edge = "y".repeat(100);
    create_token(&pool, &uid, &edge, None).await.unwrap();
}

#[tokio::test]
async fn token_limit_is_twenty_per_user() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let other = create_test_user(&pool).await;

    for index in 0..20 {
        create_token(&pool, &uid, &format!("Feed {index}"), None)
            .await
            .unwrap();
    }
    let err = create_token(&pool, &uid, "One too many", None)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        DomainError::Validation("calendar_token_limit")
    ));

    // The limit is per user.
    create_token(&pool, &other, "Other feed", None)
        .await
        .unwrap();
}

#[tokio::test]
async fn list_tokens_returns_only_owned_tokens() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let other = create_test_user(&pool).await;
    create_token(&pool, &uid, "Mine A", None).await.unwrap();
    create_token(&pool, &uid, "Mine B", None).await.unwrap();
    create_token(&pool, &other, "Theirs", None).await.unwrap();

    let tokens = list_tokens(&pool, &uid).await.unwrap();
    assert_eq!(tokens.len(), 2);
    assert!(tokens.iter().all(|token| token.user_id == uid));
}

#[tokio::test]
async fn stored_token_is_a_hash_not_the_secret() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let created = create_token(&pool, &uid, "Feed", None).await.unwrap();

    let hashes: Vec<(String,)> = sqlx::query_as("SELECT token_hash FROM calendar_tokens")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(hashes.len(), 1);
    let hash = &hashes[0].0;
    assert_eq!(hash.len(), 64);
    assert!(
        hash.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
    assert_ne!(hash, &created.token);
}

#[tokio::test]
async fn feed_marks_completed_items() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, "Sprint").await;
    let item_id = create_item(&pool, &list_id, "Ship release").await;
    sqlx::query("UPDATE items SET completed = 1 WHERE id = ?")
        .bind(&item_id)
        .execute(&pool)
        .await
        .unwrap();

    let created = create_token(&pool, &uid, "Feed", None).await.unwrap();
    let feed = feed_for_token(&pool, &created.token)
        .await
        .unwrap()
        .unwrap();
    assert!(
        feed.body.contains("SUMMARY:✓ Ship release"),
        "body:\n{}",
        feed.body
    );
}

#[tokio::test]
async fn etag_is_stable_for_same_data_and_changes_with_items() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, "Groceries").await;
    create_item(&pool, &list_id, "Buy milk").await;

    let created = create_token(&pool, &uid, "Feed", None).await.unwrap();
    let first = feed_for_token(&pool, &created.token)
        .await
        .unwrap()
        .unwrap();
    let second = feed_for_token(&pool, &created.token)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first.etag, second.etag);
    assert_eq!(first.body, second.body);
    assert_eq!(first.etag.len(), 34);

    create_item(&pool, &list_id, "Buy bread").await;
    let third = feed_for_token(&pool, &created.token)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(first.etag, third.etag);
}

#[tokio::test]
async fn touch_last_used_is_recorded_on_feed_access() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let created = create_token(&pool, &uid, "Feed", None).await.unwrap();
    feed_for_token(&pool, &created.token).await.unwrap();

    let (last_used_at,): (Option<String>,) =
        sqlx::query_as("SELECT last_used_at FROM calendar_tokens WHERE id = ?")
            .bind(&created.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(last_used_at.is_some());
}

#[tokio::test]
async fn feed_uses_the_users_timezone() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    kartoteka_db::preferences::set_timezone(&pool, &uid, "Europe/Warsaw")
        .await
        .unwrap();
    let created = create_token(&pool, &uid, "Feed", None).await.unwrap();

    let feed = feed_for_token(&pool, &created.token)
        .await
        .unwrap()
        .unwrap();
    assert!(feed.body.contains("X-WR-TIMEZONE:Europe/Warsaw"));
}

#[tokio::test]
async fn reminder_defaults_roundtrip() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;

    let custom = ReminderDefaults {
        start_date: vec![15],
        deadline: vec![120, 30],
        hard_deadline: vec![2880],
    };
    set_reminder_defaults(&pool, &uid, &custom).await.unwrap();
    assert_eq!(get_reminder_defaults(&pool, &uid).await.unwrap(), custom);
}

#[tokio::test]
async fn missing_reminder_defaults_return_the_default_set() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    assert_eq!(
        get_reminder_defaults(&pool, &uid).await.unwrap(),
        ReminderDefaults::default()
    );
}

#[tokio::test]
async fn unparsable_reminder_defaults_return_the_default_set() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    kartoteka_db::settings::set(
        &pool,
        &uid,
        kartoteka_shared::constants::REMINDER_DEFAULTS_KEY,
        "not json",
    )
    .await
    .unwrap();
    assert_eq!(
        get_reminder_defaults(&pool, &uid).await.unwrap(),
        ReminderDefaults::default()
    );
}

#[tokio::test]
async fn invalid_reminder_defaults_are_rejected() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;

    let too_many = ReminderDefaults {
        start_date: vec![1, 2, 3, 4, 5, 6],
        deadline: Vec::new(),
        hard_deadline: Vec::new(),
    };
    let err = set_reminder_defaults(&pool, &uid, &too_many)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        DomainError::Validation("invalid_reminder_defaults")
    ));

    let too_late = ReminderDefaults {
        start_date: Vec::new(),
        deadline: vec![40_321],
        hard_deadline: Vec::new(),
    };
    let err = set_reminder_defaults(&pool, &uid, &too_late)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        DomainError::Validation("invalid_reminder_defaults")
    ));
}

#[tokio::test]
async fn feed_applies_stored_reminders() {
    let pool = test_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = create_list(&pool, &uid, "Groceries").await;
    create_item(&pool, &list_id, "Buy milk").await;

    let custom = ReminderDefaults {
        start_date: vec![15],
        deadline: Vec::new(),
        hard_deadline: Vec::new(),
    };
    set_reminder_defaults(&pool, &uid, &custom).await.unwrap();

    let created = create_token(&pool, &uid, "Feed", None).await.unwrap();
    let feed = feed_for_token(&pool, &created.token)
        .await
        .unwrap()
        .unwrap();
    assert!(feed.body.contains("TRIGGER:-PT15M"), "body:\n{}", feed.body);
}
