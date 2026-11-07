//! Helpers shared by the MCP tests: in-memory pools, request extensions, a
//! `KartotekaServer` built on the real locales, and input builders.
//!
//! Lives behind `#[cfg(test)]` so the per-area tests in `crate::tools::*` can
//! reuse it without touching production code.
#![allow(dead_code)]

use super::{KartotekaServer, annotations};
use crate::McpI18n;
use crate::tools::containers::CreateContainersInput;
use crate::tools::items::CreateItemsInput;
use crate::tools::lists::{CreateListsInput, CreateListsParams};
use http::request::Parts;
use kartoteka_db::lists::{InsertListInput, insert as insert_list};
use kartoteka_domain as domain;
use kartoteka_shared::auth_ctx::{UserId, UserLocale};
use kartoteka_shared::types::CreateContainerRequest;
use rmcp::model::{CallToolResult, ContentBlock};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::str::FromStr;
use std::sync::Arc;

pub(crate) async fn single_conn_pool() -> SqlitePool {
    let name = uuid::Uuid::new_v4().simple().to_string();
    let url = format!("file:{name}?mode=memory&cache=shared");
    let options = SqliteConnectOptions::from_str(&url)
        .unwrap()
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    kartoteka_db::run_migrations(&pool).await.unwrap();
    pool
}

/// Multi-connection in-memory pool for tests that hold a transaction while also
/// querying the pool (which deadlocks on a single-connection pool).
pub(crate) async fn multi_conn_pool() -> SqlitePool {
    let name = uuid::Uuid::new_v4().simple().to_string();
    let url = format!("file:{name}?mode=memory&cache=shared");
    let options = SqliteConnectOptions::from_str(&url)
        .unwrap()
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await
        .unwrap();
    kartoteka_db::run_migrations(&pool).await.unwrap();
    pool
}

pub(crate) fn parts_with_extensions(extensions: http::Extensions) -> Parts {
    let mut req = http::Request::new(());
    *req.extensions_mut() = extensions;
    let (parts, _) = req.into_parts();
    parts
}

pub(crate) fn test_server(pool: SqlitePool) -> KartotekaServer {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let locales = manifest.parent().unwrap().parent().unwrap().join("locales");
    KartotekaServer::new(pool, Arc::new(McpI18n::load_from(&locales)))
}

pub(crate) fn parts_for_user(user_id: &str) -> Parts {
    let mut ext = http::Extensions::new();
    ext.insert(UserId(user_id.into()));
    parts_with_extensions(ext)
}

pub(crate) async fn insert_test_list(pool: &SqlitePool, user_id: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let mut tx = pool.begin().await.unwrap();
    insert_list(
        &mut tx,
        &InsertListInput {
            id: id.clone(),
            user_id: user_id.to_owned(),
            position: 0,
            name: "Test List".to_owned(),
            list_type: "checklist".to_owned(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    id
}

/// Like [`insert_test_list`], but with the given feature names enabled.
pub(crate) async fn insert_test_list_with_features(
    pool: &SqlitePool,
    user_id: &str,
    features: &[&str],
) -> String {
    let id = insert_test_list(pool, user_id).await;
    let obj: serde_json::Map<String, serde_json::Value> = features
        .iter()
        .map(|n| ((*n).to_string(), serde_json::json!({})))
        .collect();
    sqlx::query("UPDATE lists SET features = ? WHERE id = ?")
        .bind(serde_json::to_string(&obj).unwrap())
        .bind(&id)
        .execute(pool)
        .await
        .unwrap();
    id
}

/// Snapshot row for one tool: name, description key and the annotation
/// hints `list_tools` publishes. One line per tool so a diff names exactly
/// what changed.
pub(crate) fn tool_row(tool: &rmcp::model::Tool) -> String {
    let name = tool.name.as_ref();
    let ann = annotations::for_tool(name);
    format!(
        "{name} | desc={} | ro={:?} destructive={:?} idempotent={:?} open_world={:?}",
        tool.description.as_deref().unwrap_or("<none>"),
        ann.read_only_hint,
        ann.destructive_hint,
        ann.idempotent_hint,
        ann.open_world_hint,
    )
}

pub(crate) fn tool_rows(server: &KartotekaServer) -> Vec<String> {
    let mut rows: Vec<String> = server.tool_router.list_all().iter().map(tool_row).collect();
    rows.sort();
    rows
}

/// Extract the JSON payload from a tool result and render it as text.
pub(crate) fn result_text(result: &CallToolResult) -> String {
    result
        .content
        .iter()
        .find_map(|c| match c {
            ContentBlock::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .expect("text content")
}

pub(crate) fn item_input(title: &str) -> CreateItemsInput {
    CreateItemsInput {
        title: title.to_owned(),
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
    }
}

pub(crate) fn list_input(name: &str, list_type: Option<&str>) -> CreateListsInput {
    CreateListsInput {
        name: name.to_owned(),
        list_type: list_type.map(str::to_owned),
        icon: None,
        color: None,
        description: None,
        features: None,
        container_id: None,
        container_ref: None,
        parent_list_id: None,
        parent_list_ref: None,
        client_ref: None,
    }
}

pub(crate) fn container_input(
    name: &str,
    parent_container_id: Option<String>,
) -> CreateContainersInput {
    CreateContainersInput {
        name: name.to_owned(),
        icon: None,
        color: None,
        description: None,
        status: None,
        parent_container_id,
        parent_container_ref: None,
        client_ref: None,
    }
}

pub(crate) fn parts_for_user_locale(user_id: &str, locale: &str) -> Parts {
    let mut ext = http::Extensions::new();
    ext.insert(UserId(user_id.into()));
    ext.insert(UserLocale(locale.into()));
    parts_with_extensions(ext)
}

pub(crate) fn domain_container_req(name: &str) -> CreateContainerRequest {
    CreateContainerRequest {
        name: name.to_owned(),
        icon: None,
        color: None,
        description: None,
        status: None,
        parent_container_id: None,
    }
}

pub(crate) fn domain_list_req(
    name: &str,
    container_id: Option<String>,
) -> domain::lists::CreateListRequest {
    domain::lists::CreateListRequest {
        name: name.to_owned(),
        list_type: None,
        icon: None,
        color: None,
        description: None,
        container_id,
        parent_list_id: None,
        features: Vec::new(),
    }
}

pub(crate) fn single_list_params(name: &str, parent_list_id: Option<String>) -> CreateListsParams {
    CreateListsParams {
        lists: vec![CreateListsInput {
            name: name.to_owned(),
            list_type: None,
            icon: None,
            color: None,
            description: None,
            features: None,
            container_id: None,
            container_ref: None,
            parent_list_id,
            parent_list_ref: None,
            client_ref: None,
        }],
    }
}

// ── status / icons / colours ──────────────────────────────────────────────
pub(crate) async fn item_json(result: &CallToolResult) -> serde_json::Value {
    serde_json::from_str(&result_text(result)).unwrap()
}
