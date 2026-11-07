//! Tests for the MCP server core: opaque cursors, locale extraction, the
//! `map_err` table, and end-to-end behaviour of individual tools.

use super::test_support::*;
use super::{KartotekaServer, annotations, decode_cursor, encode_cursor};
use crate::McpError;
use crate::tools::comments::AddCommentParams;
use crate::tools::containers::{
    CreateContainerParams, CreateContainersInput, CreateContainersParams, SetContainerStatusParams,
};
use crate::tools::items::{CreateItemsInput, CreateItemsParams, SetItemStatusParams};
use crate::tools::lists::{CreateListsInput, CreateListsParams, MoveListToContainerParams};
use crate::tools::search::SearchItemsParams;
use crate::tools::tags::{CreateTagParams, CreateTagsInput, CreateTagsParams};
use kartoteka_db as db;
use kartoteka_db::test_helpers::create_test_user;
use kartoteka_domain as domain;
use kartoteka_shared::auth_ctx::{UserId, UserLocale};
use kartoteka_shared::types::CreateContainerRequest;
use rmcp::handler::server::tool::Extension;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::ContentBlock;
use serde_json::json;

/// Guard (#383): the refactor must not add, drop, rename or re-describe a
/// tool, and every tool keeps its annotation hints. Regenerate only when a
/// tool is added or its published metadata changes on purpose.
#[tokio::test]
async fn tool_router_matches_the_published_snapshot() {
    const EXPECTED: &[&str] = &[
        "add_comment | desc=mcp-tool-add_comment-desc | ro=Some(false) destructive=Some(false) idempotent=Some(false) open_world=Some(false)",
        "add_relation | desc=mcp-tool-add_relation-desc | ro=Some(false) destructive=Some(false) idempotent=Some(false) open_world=Some(false)",
        "assign_tag | desc=mcp-tool-assign_tag-desc | ro=Some(false) destructive=Some(false) idempotent=Some(false) open_world=Some(false)",
        "create_container | desc=mcp-tool-create_container-desc | ro=Some(false) destructive=Some(false) idempotent=Some(false) open_world=Some(false)",
        "create_containers | desc=mcp-tool-create_containers-desc | ro=Some(false) destructive=Some(false) idempotent=Some(false) open_world=Some(false)",
        "create_item | desc=mcp-tool-create_item-desc | ro=Some(false) destructive=Some(false) idempotent=Some(false) open_world=Some(false)",
        "create_items | desc=mcp-tool-create_items-desc | ro=Some(false) destructive=Some(false) idempotent=Some(false) open_world=Some(false)",
        "create_list | desc=mcp-tool-create_list-desc | ro=Some(false) destructive=Some(false) idempotent=Some(false) open_world=Some(false)",
        "create_list_from_template | desc=mcp-tool-create_list_from_template-desc | ro=Some(false) destructive=Some(false) idempotent=Some(false) open_world=Some(false)",
        "create_lists | desc=mcp-tool-create_lists-desc | ro=Some(false) destructive=Some(false) idempotent=Some(false) open_world=Some(false)",
        "create_tag | desc=mcp-tool-create_tag-desc | ro=Some(false) destructive=Some(false) idempotent=Some(false) open_world=Some(false)",
        "create_tags | desc=mcp-tool-create_tags-desc | ro=Some(false) destructive=Some(false) idempotent=Some(false) open_world=Some(false)",
        "get_active_timer | desc=mcp-tool-get_active_timer-desc | ro=Some(true) destructive=Some(false) idempotent=Some(true) open_world=Some(false)",
        "get_container | desc=mcp-tool-get_container-desc | ro=Some(true) destructive=Some(false) idempotent=Some(true) open_world=Some(false)",
        "get_item | desc=mcp-tool-get_item-desc | ro=Some(true) destructive=Some(false) idempotent=Some(true) open_world=Some(false)",
        "get_list | desc=mcp-tool-get_list-desc | ro=Some(true) destructive=Some(false) idempotent=Some(true) open_world=Some(false)",
        "get_relations | desc=mcp-tool-get_relations-desc | ro=Some(true) destructive=Some(false) idempotent=Some(true) open_world=Some(false)",
        "get_tag_entities | desc=mcp-tool-get_tag_entities-desc | ro=Some(true) destructive=Some(false) idempotent=Some(true) open_world=Some(false)",
        "get_time_summary | desc=mcp-tool-get_time_summary-desc | ro=Some(true) destructive=Some(false) idempotent=Some(true) open_world=Some(false)",
        "get_today | desc=mcp-tool-get_today-desc | ro=Some(true) destructive=Some(false) idempotent=Some(true) open_world=Some(false)",
        "list_container_items | desc=mcp-tool-list_container_items-desc | ro=Some(true) destructive=Some(false) idempotent=Some(true) open_world=Some(false)",
        "list_containers | desc=mcp-tool-list_containers-desc | ro=Some(true) destructive=Some(false) idempotent=Some(true) open_world=Some(false)",
        "list_items | desc=mcp-tool-list_items-desc | ro=Some(true) destructive=Some(false) idempotent=Some(true) open_world=Some(false)",
        "list_lists | desc=mcp-tool-list_lists-desc | ro=Some(true) destructive=Some(false) idempotent=Some(true) open_world=Some(false)",
        "list_marked_for_delete | desc=mcp-tool-list_marked_for_delete-desc | ro=Some(true) destructive=Some(false) idempotent=Some(true) open_world=Some(false)",
        "list_overdue | desc=mcp-tool-list_overdue-desc | ro=Some(true) destructive=Some(false) idempotent=Some(true) open_world=Some(false)",
        "list_tags | desc=mcp-tool-list_tags-desc | ro=Some(true) destructive=Some(false) idempotent=Some(true) open_world=Some(false)",
        "list_templates | desc=mcp-tool-list_templates-desc | ro=Some(true) destructive=Some(false) idempotent=Some(true) open_world=Some(false)",
        "log_time | desc=mcp-tool-log_time-desc | ro=Some(false) destructive=Some(false) idempotent=Some(false) open_world=Some(false)",
        "move_container_to_parent | desc=mcp-tool-move_container_to_parent-desc | ro=Some(false) destructive=Some(true) idempotent=Some(false) open_world=Some(false)",
        "move_list_to_container | desc=mcp-tool-move_list_to_container-desc | ro=Some(false) destructive=Some(true) idempotent=Some(false) open_world=Some(false)",
        "remove_relation | desc=mcp-tool-remove_relation-desc | ro=Some(false) destructive=Some(true) idempotent=Some(false) open_world=Some(false)",
        "save_as_template | desc=mcp-tool-save_as_template-desc | ro=Some(false) destructive=Some(false) idempotent=Some(false) open_world=Some(false)",
        "search_items | desc=mcp-tool-search_items-desc | ro=Some(true) destructive=Some(false) idempotent=Some(true) open_world=Some(false)",
        "set_container_status | desc=mcp-tool-set_container_status-desc | ro=Some(false) destructive=Some(true) idempotent=Some(false) open_world=Some(false)",
        "set_item_status | desc=mcp-tool-set_item_status-desc | ro=Some(false) destructive=Some(true) idempotent=Some(false) open_world=Some(false)",
        "start_timer | desc=mcp-tool-start_timer-desc | ro=Some(false) destructive=Some(false) idempotent=Some(false) open_world=Some(false)",
        "stop_timer | desc=mcp-tool-stop_timer-desc | ro=Some(false) destructive=Some(true) idempotent=Some(false) open_world=Some(false)",
        "toggle_item | desc=mcp-tool-toggle_item-desc | ro=Some(false) destructive=Some(true) idempotent=Some(false) open_world=Some(false)",
        "toggle_mark_for_delete | desc=mcp-tool-toggle_mark_for_delete-desc | ro=Some(false) destructive=Some(false) idempotent=Some(false) open_world=Some(false)",
        "unassign_tag | desc=mcp-tool-unassign_tag-desc | ro=Some(false) destructive=Some(true) idempotent=Some(false) open_world=Some(false)",
        "update_item | desc=mcp-tool-update_item-desc | ro=Some(false) destructive=Some(true) idempotent=Some(false) open_world=Some(false)",
    ];
    let server = test_server(multi_conn_pool().await);
    assert_eq!(
        tool_rows(&server),
        EXPECTED,
        "the published tool list changed"
    );
}

#[test]
fn cursor_round_trips_offsets() {
    for offset in [0, 3, 100] {
        assert_eq!(decode_cursor(Some(&encode_cursor(offset))), offset);
    }
}

#[test]
fn decode_missing_cursor_starts_at_zero() {
    assert_eq!(decode_cursor(None), 0);
}

#[test]
fn decode_legacy_plain_number_is_rejected() {
    assert_eq!(decode_cursor(Some("3")), 0);
}

#[test]
fn decode_garbage_is_rejected() {
    assert_eq!(decode_cursor(Some("!!!")), 0);
}

#[test]
fn encoded_cursor_is_opaque() {
    assert!(!encode_cursor(100).contains("100"));
}

#[test]
fn extract_returns_user_and_locale_when_present() {
    let mut ext = http::Extensions::new();
    ext.insert(UserId("alice".into()));
    ext.insert(UserLocale("pl".into()));
    let parts = parts_with_extensions(ext);

    let (uid, locale) = KartotekaServer::extract_user_id_and_locale(&parts).unwrap();
    assert_eq!(uid, "alice");
    assert_eq!(locale, "pl");
}

#[test]
fn extract_falls_back_to_en_when_locale_missing() {
    let mut ext = http::Extensions::new();
    ext.insert(UserId("bob".into()));
    let parts = parts_with_extensions(ext);

    let (uid, locale) = KartotekaServer::extract_user_id_and_locale(&parts).unwrap();
    assert_eq!(uid, "bob");
    assert_eq!(locale, "en");
}

#[test]
fn extract_errors_when_user_id_missing() {
    let parts = parts_with_extensions(http::Extensions::new());
    let err = KartotekaServer::extract_user_id_and_locale(&parts).unwrap_err();
    assert!(matches!(err, McpError::Unauthorized));
}

#[tokio::test]
async fn create_items_with_deadline_on_list_without_deadlines_feature_is_rejected() {
    let pool = multi_conn_pool().await;
    let uid = create_test_user(&pool).await;
    // insert_test_list seeds an empty feature set, so dates are not allowed.
    let list_id = insert_test_list(&pool, &uid).await;
    let server = test_server(pool);

    let mut dated = item_input("Call mom");
    dated.deadline = Some("2030-01-31".to_owned());
    let params = CreateItemsParams {
        list_id: list_id.clone(),
        items: vec![item_input("Fine"), dated],
    };

    let err = match server
        .create_items(Extension(parts_for_user(&uid)), Parameters(params))
        .await
    {
        Ok(_) => panic!("create_items must reject dates on a list without the feature"),
        Err(e) => e,
    };
    assert!(
        err.message.contains("deadlines"),
        "unexpected message: {}",
        err.message
    );

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE list_id = ?")
        .bind(&list_id)
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0, "a rejected batch must not write anything");
}

#[tokio::test]
async fn create_items_with_unparsable_date_is_rejected() {
    let pool = multi_conn_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = insert_test_list(&pool, &uid).await;
    let server = test_server(pool);

    let mut broken = item_input("Bad date");
    broken.start_date = Some("garbage".to_owned());
    let params = CreateItemsParams {
        list_id: list_id.clone(),
        items: vec![broken],
    };

    let err = match server
        .create_items(Extension(parts_for_user(&uid)), Parameters(params))
        .await
    {
        Ok(_) => panic!("create_items must reject an unparsable date"),
        Err(e) => e,
    };
    assert!(
        err.message.contains("invalid_date"),
        "unexpected message: {}",
        err.message
    );
}

#[tokio::test]
async fn create_lists_with_unknown_list_type_is_rejected() {
    let pool = multi_conn_pool().await;
    let uid = create_test_user(&pool).await;
    let server = test_server(pool);

    let params = CreateListsParams {
        lists: vec![list_input("Bad type", Some("custom"))],
    };

    let err = match server
        .create_lists(Extension(parts_for_user(&uid)), Parameters(params))
        .await
    {
        Ok(_) => panic!("create_lists must reject an unknown list_type"),
        Err(e) => e,
    };
    assert!(
        err.message.contains("unknown_list_type"),
        "unexpected message: {}",
        err.message
    );

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM lists WHERE user_id = ?")
        .bind(&uid)
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0, "a rejected batch must not write anything");
}

#[tokio::test]
async fn create_lists_defaults_to_checklist_with_default_features() {
    let pool = multi_conn_pool().await;
    let uid = create_test_user(&pool).await;
    let server = test_server(pool);

    let params = CreateListsParams {
        lists: vec![list_input("Defaulted", None)],
    };
    let result = server
        .create_lists(Extension(parts_for_user(&uid)), Parameters(params))
        .await
        .unwrap();

    let lists: Vec<serde_json::Value> = serde_json::from_str(&result_text(&result)).unwrap();
    let id = lists[0]["id"].as_str().unwrap();

    let row: (String, String) =
        sqlx::query_as("SELECT list_type, features FROM lists WHERE id = ?")
            .bind(id)
            .fetch_one(&server.pool)
            .await
            .unwrap();
    assert_eq!(row.0, "checklist");
    let features: serde_json::Value = serde_json::from_str(&row.1).unwrap();
    assert!(
        features.get("checklist").is_some(),
        "default features missing: {}",
        row.1
    );
}

#[tokio::test]
async fn create_containers_rejects_non_folder_parent() {
    let pool = multi_conn_pool().await;
    let uid = create_test_user(&pool).await;
    let server = test_server(pool);

    let project = domain::containers::create(
        &server.pool,
        &uid,
        &CreateContainerRequest {
            name: "Project".to_owned(),
            icon: None,
            description: None,
            status: Some("active".to_owned()),
            parent_container_id: None,
            color: None,
        },
    )
    .await
    .unwrap();

    let params = CreateContainersParams {
        containers: vec![container_input("Child", Some(project.id.clone()))],
    };

    let err = match server
        .create_containers(Extension(parts_for_user(&uid)), Parameters(params))
        .await
    {
        Ok(_) => panic!("create_containers must reject a non-folder parent"),
        Err(e) => e,
    };
    assert!(
        err.message.contains("invalid_container_hierarchy"),
        "unexpected message: {}",
        err.message
    );

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM containers WHERE user_id = ?")
        .bind(&uid)
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1, "only the pre-existing project should remain");
}

#[tokio::test]
async fn create_containers_allows_folder_parent_from_same_batch() {
    let pool = multi_conn_pool().await;
    let uid = create_test_user(&pool).await;
    let server = test_server(pool);

    let params = CreateContainersParams {
        containers: vec![
            CreateContainersInput {
                client_ref: Some("folder".to_owned()),
                ..container_input("Folder", None)
            },
            CreateContainersInput {
                parent_container_ref: Some("folder".to_owned()),
                ..container_input("Child", None)
            },
        ],
    };

    let result = server
        .create_containers(Extension(parts_for_user(&uid)), Parameters(params))
        .await
        .unwrap();
    let created: Vec<serde_json::Value> = serde_json::from_str(&result_text(&result)).unwrap();
    assert_eq!(created.len(), 2);

    let child_id = created[1]["id"].as_str().unwrap();
    let row: (Option<String>,) =
        sqlx::query_as("SELECT parent_container_id FROM containers WHERE id = ?")
            .bind(child_id)
            .fetch_one(&server.pool)
            .await
            .unwrap();
    assert_eq!(row.0.as_deref(), Some(created[0]["id"].as_str().unwrap()));
}

#[tokio::test]
async fn create_tags_rejects_foreign_parent_tag() {
    let pool = multi_conn_pool().await;
    let owner = create_test_user(&pool).await;
    let intruder = create_test_user(&pool).await;
    let server = test_server(pool);

    let parent = domain::tags::create(
        &server.pool,
        &owner,
        &domain::tags::CreateTagRequest {
            name: "Work".to_owned(),
            icon: None,
            color: None,
            parent_tag_id: None,
            tag_type: None,
            metadata: None,
        },
    )
    .await
    .unwrap();

    let params = CreateTagsParams {
        tags: vec![CreateTagsInput {
            name: "Child".to_owned(),
            color: None,
            icon: None,
            parent_tag_id: Some(parent.id.clone()),
            parent_tag_ref: None,
            client_ref: None,
        }],
    };

    let err = match server
        .create_tags(Extension(parts_for_user(&intruder)), Parameters(params))
        .await
    {
        Ok(_) => panic!("create_tags must reject a foreign parent tag"),
        Err(e) => e,
    };
    assert!(
        err.message.contains("parent_tag"),
        "unexpected message: {}",
        err.message
    );

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tags WHERE user_id = ?")
        .bind(&intruder)
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0);
}

#[tokio::test]
async fn create_items_within_batch_duplicate_returns_existed() {
    let pool = multi_conn_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = insert_test_list(&pool, &uid).await;
    let server = test_server(pool);

    let params = CreateItemsParams {
        list_id: list_id.clone(),
        items: vec![
            CreateItemsInput {
                title: "Buy milk".to_owned(),
                description: None,
                icon: None,
                start_date: None,
                deadline: None,
                hard_deadline: None,
                start_time: None,
                deadline_time: None,
                quantity: None,
                actual_quantity: None,
                unit: None,
                estimated_duration: None,
                recurrence_rule: None,
                recurrence_mode: None,
            },
            CreateItemsInput {
                title: "BUY MILK".to_owned(), // same title, different case
                description: None,
                icon: None,
                start_date: None,
                deadline: None,
                hard_deadline: None,
                start_time: None,
                deadline_time: None,
                quantity: None,
                actual_quantity: None,
                unit: None,
                estimated_duration: None,
                recurrence_rule: None,
                recurrence_mode: None,
            },
        ],
    };

    let result = server
        .create_items(Extension(parts_for_user(&uid)), Parameters(params))
        .await
        .unwrap();

    let text = result
        .content
        .into_iter()
        .find_map(|c| match c {
            ContentBlock::Text(t) => Some(t.text),
            _ => None,
        })
        .expect("text content");
    let items: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap();

    assert_eq!(items.len(), 2, "both inputs must produce an output");

    let first_id = items[0]["id"].as_str().unwrap();
    assert!(
        items[0].get("existed").is_none(),
        "first item should be a new insert"
    );
    assert_eq!(
        items[1]["id"].as_str().unwrap(),
        first_id,
        "duplicate should reference the first item's id"
    );
    assert_eq!(
        items[1]["existed"].as_bool(),
        Some(true),
        "duplicate must carry existed=true"
    );

    // Only one row inserted
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE list_id = ?")
        .bind(&list_id)
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1, "only one item row should exist in the DB");
}

#[tokio::test]
async fn create_items_no_deadlock_single_conn() {
    let pool = single_conn_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = insert_test_list(&pool, &uid).await;
    let server = test_server(pool);

    let params = CreateItemsParams {
        list_id: list_id.clone(),
        items: vec![
            CreateItemsInput {
                title: "Alpha".to_owned(),
                description: None,
                icon: None,
                start_date: None,
                deadline: None,
                hard_deadline: None,
                start_time: None,
                deadline_time: None,
                quantity: None,
                actual_quantity: None,
                unit: None,
                estimated_duration: None,
                recurrence_rule: None,
                recurrence_mode: None,
            },
            CreateItemsInput {
                title: "Beta".to_owned(),
                description: None,
                icon: None,
                start_date: None,
                deadline: None,
                hard_deadline: None,
                start_time: None,
                deadline_time: None,
                quantity: None,
                actual_quantity: None,
                unit: None,
                estimated_duration: None,
                recurrence_rule: None,
                recurrence_mode: None,
            },
        ],
    };

    let result = server
        .create_items(Extension(parts_for_user(&uid)), Parameters(params))
        .await
        .unwrap();

    let text = result
        .content
        .into_iter()
        .find_map(|c| match c {
            ContentBlock::Text(t) => Some(t.text),
            _ => None,
        })
        .expect("text content");
    let items: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap();

    assert_eq!(items.len(), 2);
    assert!(items[0].get("existed").is_none());
    assert!(items[1].get("existed").is_none());

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items WHERE list_id = ?")
        .bind(&list_id)
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(count.0, 2);
}

#[tokio::test]
async fn create_lists_no_deadlock_single_conn() {
    let pool = single_conn_pool().await;
    let uid = create_test_user(&pool).await;
    let server = test_server(pool);

    let params = CreateListsParams {
        lists: vec![
            CreateListsInput {
                name: "List One".to_owned(),
                list_type: None,
                icon: None,
                color: None,
                description: None,
                features: None,
                container_id: None,
                container_ref: None,
                parent_list_id: None,
                parent_list_ref: None,
                client_ref: None,
            },
            CreateListsInput {
                name: "List Two".to_owned(),
                list_type: None,
                icon: None,
                color: None,
                description: None,
                features: None,
                container_id: None,
                container_ref: None,
                parent_list_id: None,
                parent_list_ref: None,
                client_ref: None,
            },
        ],
    };

    let result = server
        .create_lists(Extension(parts_for_user(&uid)), Parameters(params))
        .await
        .unwrap();

    let text = result
        .content
        .into_iter()
        .find_map(|c| match c {
            ContentBlock::Text(t) => Some(t.text),
            _ => None,
        })
        .expect("text content");
    let lists: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap();

    assert_eq!(lists.len(), 2);
    assert!(lists[0].get("existed").is_none());
    assert!(lists[1].get("existed").is_none());

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM lists WHERE user_id = ?")
        .bind(&uid)
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(count.0, 2);
}

#[tokio::test]
async fn create_lists_under_own_parent_list_ok() {
    let pool = multi_conn_pool().await;
    let uid = create_test_user(&pool).await;
    let parent = insert_test_list(&pool, &uid).await;
    let server = test_server(pool);

    let result = server
        .create_lists(
            Extension(parts_for_user(&uid)),
            Parameters(single_list_params("Child", Some(parent.clone()))),
        )
        .await
        .unwrap();
    let text = result
        .content
        .into_iter()
        .find_map(|c| match c {
            ContentBlock::Text(t) => Some(t.text),
            _ => None,
        })
        .expect("text content");
    let lists: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap();
    let child_id = lists[0]["id"].as_str().unwrap();

    let row: (Option<String>,) = sqlx::query_as("SELECT parent_list_id FROM lists WHERE id = ?")
        .bind(child_id)
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(row.0.as_deref(), Some(parent.as_str()));
}

#[tokio::test]
async fn create_lists_with_other_users_parent_list_is_rejected() {
    let pool = multi_conn_pool().await;
    let owner = create_test_user(&pool).await;
    let intruder = create_test_user(&pool).await;
    let foreign_parent = insert_test_list(&pool, &owner).await;
    let server = test_server(pool);

    let err = match server
        .create_lists(
            Extension(parts_for_user(&intruder)),
            Parameters(single_list_params("Child", Some(foreign_parent))),
        )
        .await
    {
        Ok(_) => panic!("create_lists must reject a foreign parent_list_id"),
        Err(e) => e,
    };
    assert!(
        err.message.contains("parent_list_id"),
        "unexpected message: {}",
        err.message
    );

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM lists WHERE user_id = ?")
        .bind(&intruder)
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0);
}

// ── map_err: error codes and machine-readable data (#375, #374) ──────────
#[tokio::test]
async fn map_err_gives_each_variant_its_own_code_and_data() {
    let server = test_server(multi_conn_pool().await);
    let serde_err = serde_json::from_str::<i32>("not a number").unwrap_err();

    let cases: Vec<(McpError, i32, Option<serde_json::Value>)> = vec![
        (
            McpError::Unauthorized,
            -32600,
            Some(json!({"error": "unauthorized"})),
        ),
        (
            McpError::Domain(domain::DomainError::NotFound("list")),
            -32602,
            Some(json!({"error": "not_found", "entity": "list"})),
        ),
        (
            McpError::Domain(domain::DomainError::Validation("duplicate_id")),
            -32602,
            Some(json!({"error": "validation", "reason": "duplicate_id"})),
        ),
        (
            McpError::Domain(domain::DomainError::FeatureRequired("deadlines")),
            -32602,
            Some(json!({"error": "feature_required", "feature": "deadlines"})),
        ),
        (
            McpError::Domain(domain::DomainError::AlreadyExists {
                kind: "list",
                id: "existing-id".to_owned(),
            }),
            -32602,
            Some(json!({"error": "already_exists", "kind": "list", "id": "existing-id"})),
        ),
        (
            McpError::Domain(domain::DomainError::NotEmpty {
                kind: "list",
                items: 3,
            }),
            -32600,
            Some(json!({"error": "not_empty", "kind": "list", "items": 3})),
        ),
        (
            McpError::Domain(domain::DomainError::HasChildren {
                kind: "container",
                lists: 2,
                containers: 1,
            }),
            -32600,
            Some(json!({
                "error": "has_children",
                "kind": "container",
                "lists": 2,
                "containers": 1,
            })),
        ),
        (
            McpError::Domain(domain::DomainError::Forbidden),
            -32600,
            Some(json!({"error": "forbidden"})),
        ),
        (
            McpError::BadRequest("one or more container_id values not found".to_owned()),
            -32602,
            Some(json!({
                "error": "validation",
                "reason": "one or more container_id values not found",
            })),
        ),
        (
            McpError::Domain(domain::DomainError::Internal(
                "db exploded at /srv/secret".to_owned(),
            )),
            -32603,
            None,
        ),
        (
            McpError::Domain(domain::DomainError::Db(db::DbError::NotFound("item"))),
            -32603,
            None,
        ),
        (McpError::Serde(serde_err), -32603, None),
    ];

    for (err, expected_code, expected_data) in cases {
        let mapped = server.map_err(err, "en");
        assert_eq!(
            mapped.code.0, expected_code,
            "wrong code for {:?}: {mapped:?}",
            mapped.message
        );
        assert_eq!(mapped.data, expected_data, "wrong data: {mapped:?}");
    }
}

#[tokio::test]
async fn map_err_internal_errors_do_not_leak_details() {
    let server = test_server(multi_conn_pool().await);
    let cause = "db exploded at /srv/secret";

    for (locale, expected_message) in [("en", "Internal error."), ("pl", "Błąd wewnętrzny.")] {
        let mapped = server.map_err(
            McpError::Domain(domain::DomainError::Internal(cause.to_owned())),
            locale,
        );
        assert_eq!(mapped.code.0, -32603);
        assert_eq!(mapped.message, expected_message, "locale {locale}");
        assert!(
            !mapped.message.contains(cause),
            "internal cause leaked into the message: {}",
            mapped.message
        );
        assert!(mapped.data.is_none());
    }
}

/// #374: moving a list onto a name that already exists in the target
/// container must tell the client which entity collides, in both locales.
#[tokio::test]
async fn move_list_to_container_with_duplicate_name_reports_already_exists() {
    let pool = multi_conn_pool().await;
    let uid = create_test_user(&pool).await;
    let server = test_server(pool);

    let container_a = domain::containers::create(&server.pool, &uid, &domain_container_req("Maj"))
        .await
        .unwrap();
    let container_b =
        domain::containers::create(&server.pool, &uid, &domain_container_req("Śmietnik"))
            .await
            .unwrap();

    let list_a = domain::lists::create(
        &server.pool,
        &uid,
        &domain_list_req("Wakacje 2026", Some(container_a.id.clone())),
    )
    .await
    .unwrap();
    let list_b = domain::lists::create(
        &server.pool,
        &uid,
        &domain_list_req("Wakacje 2026", Some(container_b.id.clone())),
    )
    .await
    .unwrap();

    for locale in ["en", "pl"] {
        let err = server
            .move_list_to_container(
                Extension(parts_for_user_locale(&uid, locale)),
                Parameters(MoveListToContainerParams {
                    list_id: list_a.id.clone(),
                    container_id: Some(container_b.id.clone()),
                }),
            )
            .await
            .unwrap_err();

        assert_eq!(err.code.0, -32602, "[{locale}] unexpected code: {err:?}");
        let data = err.data.as_ref().expect("already_exists must carry data");
        assert_eq!(data["error"], "already_exists", "[{locale}] {data}");
        assert_eq!(data["kind"], "list", "[{locale}] {data}");
        assert_eq!(
            data["id"],
            list_b.id.as_str(),
            "[{locale}] must point at the colliding list: {data}"
        );
        for leaked in ["Internal", "wewnętrzny"] {
            assert!(
                !err.message.contains(leaked),
                "[{locale}] message must not contain {leaked:?}: {}",
                err.message
            );
        }
    }

    // The move must not have gone through.
    let row: (Option<String>,) = sqlx::query_as("SELECT container_id FROM lists WHERE id = ?")
        .bind(&list_a.id)
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(row.0.as_deref(), Some(container_a.id.as_str()));
}

#[tokio::test]
async fn closed_pool_reports_internal_error() {
    let pool = multi_conn_pool().await;
    let uid = create_test_user(&pool).await;
    let server = test_server(pool.clone());
    pool.close().await;

    let err = server
        .search_items(
            Extension(parts_for_user(&uid)),
            Parameters(SearchItemsParams {
                query: "anything".to_owned(),
            }),
        )
        .await
        .unwrap_err();

    assert_eq!(err.code.0, -32603, "unexpected code: {err:?}");
    assert_eq!(err.message, "Internal error.", "locale en");
    assert!(err.data.is_none(), "internal errors carry no data");
}

#[tokio::test]
async fn create_item_passes_icon_through() {
    let pool = multi_conn_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = insert_test_list(&pool, &uid).await;
    let server = test_server(pool);

    let mut input = item_input("Buy milk");
    input.icon = Some("lucide:milk".to_owned());
    let params = CreateItemsParams {
        list_id,
        items: vec![input],
    };
    let result = server
        .create_items(Extension(parts_for_user(&uid)), Parameters(params))
        .await
        .unwrap();
    let created: Vec<serde_json::Value> = serde_json::from_str(&result_text(&result)).unwrap();
    let id = created[0]["id"].as_str().unwrap().to_owned();

    let row: (Option<String>, String) =
        sqlx::query_as("SELECT icon, status FROM items WHERE id = ?")
            .bind(&id)
            .fetch_one(&server.pool)
            .await
            .unwrap();
    assert_eq!(row.0.as_deref(), Some("lucide:milk"));
    assert_eq!(row.1, "todo");
}

#[tokio::test]
async fn set_item_status_tool_completes_and_reopens() {
    let pool = multi_conn_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = insert_test_list_with_features(&pool, &uid, &["checklist"]).await;
    let item_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO items (id, list_id, title) VALUES (?, ?, 'Task')")
        .bind(&item_id)
        .bind(&list_id)
        .execute(&pool)
        .await
        .unwrap();
    let server = test_server(pool);

    let done = server
        .set_item_status(
            Extension(parts_for_user(&uid)),
            Parameters(SetItemStatusParams {
                item_id: item_id.clone(),
                status: "done".to_owned(),
            }),
        )
        .await
        .unwrap();
    let json = item_json(&done).await;
    assert_eq!(json["status"], "done");
    assert_eq!(json["completed"], true);

    let in_progress = server
        .set_item_status(
            Extension(parts_for_user(&uid)),
            Parameters(SetItemStatusParams {
                item_id: item_id.clone(),
                status: "in_progress".to_owned(),
            }),
        )
        .await
        .unwrap();
    let json = item_json(&in_progress).await;
    assert_eq!(json["status"], "in_progress");
    assert_eq!(json["completed"], false);
}

#[tokio::test]
async fn set_item_status_tool_rejects_unknown_status() {
    let pool = multi_conn_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = insert_test_list(&pool, &uid).await;
    let item_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO items (id, list_id, title) VALUES (?, ?, 'Task')")
        .bind(&item_id)
        .bind(&list_id)
        .execute(&pool)
        .await
        .unwrap();
    let server = test_server(pool);

    let err = match server
        .set_item_status(
            Extension(parts_for_user(&uid)),
            Parameters(SetItemStatusParams {
                item_id,
                status: "blocked".to_owned(),
            }),
        )
        .await
    {
        Ok(_) => panic!("unknown status must be rejected"),
        Err(e) => e,
    };
    assert!(
        err.message.contains("invalid_item_status"),
        "unexpected message: {}",
        err.message
    );
}

#[tokio::test]
async fn set_container_status_tool_toggles_folder_and_project() {
    let pool = multi_conn_pool().await;
    let uid = create_test_user(&pool).await;
    let server = test_server(pool);

    let created = server
        .create_container(
            Extension(parts_for_user(&uid)),
            Parameters(CreateContainerParams {
                name: "Maybe".to_owned(),
                icon: None,
                color: Some("#00ff88".to_owned()),
                description: None,
                status: None,
                parent_container_id: None,
            }),
        )
        .await
        .unwrap();
    let json = item_json(&created).await;
    assert_eq!(json["color"], "#00ff88");
    let container_id = json["id"].as_str().unwrap().to_owned();

    let project = server
        .set_container_status(
            Extension(parts_for_user(&uid)),
            Parameters(SetContainerStatusParams {
                container_id: container_id.clone(),
                status: Some("active".to_owned()),
            }),
        )
        .await
        .unwrap();
    assert_eq!(item_json(&project).await["status"], "active");

    let folder = server
        .set_container_status(
            Extension(parts_for_user(&uid)),
            Parameters(SetContainerStatusParams {
                container_id,
                status: None,
            }),
        )
        .await
        .unwrap();
    assert!(item_json(&folder).await["status"].is_null());
}

#[tokio::test]
async fn create_tag_passes_icon_through() {
    let pool = multi_conn_pool().await;
    let uid = create_test_user(&pool).await;
    let server = test_server(pool);

    let created = server
        .create_tag(
            Extension(parts_for_user(&uid)),
            Parameters(CreateTagParams {
                name: "urgent".to_owned(),
                color: Some("#ff0000".to_owned()),
                icon: Some("lucide:flame".to_owned()),
                parent_tag_id: None,
            }),
        )
        .await
        .unwrap();
    assert_eq!(item_json(&created).await["icon"], "lucide:flame");
}

#[tokio::test]
async fn create_tags_passes_icon_through() {
    let pool = multi_conn_pool().await;
    let uid = create_test_user(&pool).await;
    let server = test_server(pool);

    let params = CreateTagsParams {
        tags: vec![CreateTagsInput {
            name: "later".to_owned(),
            color: None,
            icon: Some("emoji:⏳".to_owned()),
            parent_tag_id: None,
            parent_tag_ref: None,
            client_ref: None,
        }],
    };
    server
        .create_tags(Extension(parts_for_user(&uid)), Parameters(params))
        .await
        .unwrap();

    let row: (Option<String>,) = sqlx::query_as("SELECT icon FROM tags WHERE name = 'later'")
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(row.0.as_deref(), Some("emoji:⏳"));
}

#[tokio::test]
async fn add_comment_records_assistant_author() {
    let pool = multi_conn_pool().await;
    let uid = create_test_user(&pool).await;
    let list_id = insert_test_list(&pool, &uid).await;
    let server = test_server(pool);

    server
        .add_comment(
            Extension(parts_for_user(&uid)),
            Parameters(AddCommentParams {
                entity_type: "list".to_owned(),
                entity_id: list_id,
                content: "Looks good".to_owned(),
                author_name: Some("Claude".to_owned()),
            }),
        )
        .await
        .unwrap();

    let row: (String, Option<String>) =
        sqlx::query_as("SELECT author_type, author_name FROM comments")
            .fetch_one(&server.pool)
            .await
            .unwrap();
    assert_eq!(row.0, "assistant");
    assert_eq!(row.1.as_deref(), Some("Claude"));
}

// ── tool key + annotation coverage (#383) ────────────────────────────────────

/// Every published tool needs `mcp-tool-<name>-desc` in *every* locale, not just
/// the `en` fallback `McpI18n::translate` would silently use.
#[tokio::test]
async fn every_tool_has_a_description_in_every_locale() {
    let server = test_server(multi_conn_pool().await);
    let names = tool_names(&server);
    assert!(!names.is_empty(), "the router publishes no tools");

    for name in &names {
        let key = format!("mcp-tool-{name}-desc");
        let declared = server
            .tool_router
            .get(name)
            .and_then(|t| t.description.as_deref().map(str::to_owned));
        assert_eq!(
            declared.as_deref(),
            Some(key.as_str()),
            "{name} must declare its description as {key}"
        );
        for locale in ["en", "pl"] {
            assert!(
                server.i18n.has_key(locale, &key),
                "locales/{locale}/mcp.ftl is missing {key}"
            );
        }
    }
}

/// Every published tool must sit in one of the annotation buckets, so clients
/// can tell read-only from additive and destructive tools (#342).
#[tokio::test]
async fn every_tool_is_annotated() {
    let server = test_server(multi_conn_pool().await);

    for name in tool_names(&server) {
        let ann = annotations::for_tool(&name);
        assert!(
            ann.read_only_hint.is_some()
                && ann.destructive_hint.is_some()
                && ann.idempotent_hint.is_some()
                && ann.open_world_hint.is_some(),
            "{name} is in none of READ_ONLY / ADDITIVE_WRITE / DESTRUCTIVE in server/annotations.rs"
        );
    }
}
