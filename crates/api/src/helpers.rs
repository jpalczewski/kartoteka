use crate::error::json_error;
use serde::de::DeserializeOwned;
use wasm_bindgen::JsValue;
use worker::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedListState {
    pub id: String,
    pub archived: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedItemState {
    pub id: String,
    pub list_id: String,
    pub list_archived: bool,
}

pub fn dedupe_ids(ids: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    ids.iter()
        .filter(|id| seen.insert(id.as_str()))
        .cloned()
        .collect()
}

pub fn ids_match_exact_set(expected: &[String], actual: &[String]) -> bool {
    if expected.len() != actual.len() {
        return false;
    }
    if dedupe_ids(actual).len() != actual.len() {
        return false;
    }

    let expected_ids: std::collections::HashSet<&str> =
        expected.iter().map(String::as_str).collect();
    actual.iter().all(|id| expected_ids.contains(id.as_str()))
}

/// Ensure a user row exists in the `users` table. Creates one if absent.
/// If `initial_admin_email` is non-empty and matches the user's email,
/// promotes the user to admin on first creation.
/// Returns `is_admin` flag.
pub async fn ensure_user_exists(
    d1: &D1Database,
    user_id: &str,
    email: &str,
    initial_admin_email: &str,
) -> Result<bool> {
    let initial_admin = !initial_admin_email.is_empty() && email == initial_admin_email;
    let is_admin_val: i32 = if initial_admin { 1 } else { 0 };
    d1.prepare("INSERT OR IGNORE INTO users (id, email, is_admin) VALUES (?1, ?2, ?3)")
        .bind(&[user_id.into(), email.into(), is_admin_val.into()])?
        .run()
        .await?;

    let row = d1
        .prepare("SELECT is_admin FROM users WHERE id = ?1")
        .bind(&[user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row
        .and_then(|v| v.get("is_admin")?.as_f64())
        .map(|f| f != 0.0)
        .unwrap_or(false))
}

/// Returns a 403 Forbidden response if the user is not an admin.
/// Call at the top of admin handlers.
pub async fn require_admin(d1: &D1Database, user_id: &str) -> Result<Option<Response>> {
    let row = d1
        .prepare("SELECT is_admin FROM users WHERE id = ?1")
        .bind(&[user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    let is_admin = row
        .and_then(|v| v.get("is_admin")?.as_f64())
        .map(|f| f != 0.0)
        .unwrap_or(false);

    if !is_admin {
        Ok(Some(Response::error("Forbidden", 403)?))
    } else {
        Ok(None)
    }
}

/// Check if a resource belongs to the user. Returns `true` if owned.
pub async fn check_ownership(
    d1: &D1Database,
    table: &str,
    id: &str,
    user_id: &str,
) -> Result<bool> {
    let check = d1
        .prepare(format!(
            "SELECT id FROM {table} WHERE id = ?1 AND user_id = ?2"
        ))
        .bind(&[id.into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(check.is_some())
}

pub async fn get_owned_list_state(
    d1: &D1Database,
    list_id: &str,
    user_id: &str,
) -> Result<Option<OwnedListState>> {
    let row = d1
        .prepare("SELECT id, archived FROM lists WHERE id = ?1 AND user_id = ?2")
        .bind(&[list_id.into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    Ok(row.and_then(|value| {
        let id = value.get("id")?.as_str()?.to_string();
        let archived = value
            .get("archived")
            .and_then(|v| v.as_f64())
            .map(|f| f != 0.0)
            .unwrap_or(false);
        Some(OwnedListState { id, archived })
    }))
}

/// Check if an item belongs to the user (via list join). Returns `true` if owned.
pub async fn check_item_ownership(d1: &D1Database, item_id: &str, user_id: &str) -> Result<bool> {
    let check = d1
        .prepare(
            "SELECT items.id FROM items \
             JOIN lists ON lists.id = items.list_id \
             WHERE items.id = ?1 AND lists.user_id = ?2",
        )
        .bind(&[item_id.into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(check.is_some())
}

/// Check item ownership and return list metadata. Returns `None` if not owned.
pub async fn get_owned_item_state(
    d1: &D1Database,
    item_id: &str,
    user_id: &str,
) -> Result<Option<OwnedItemState>> {
    let check = d1
        .prepare(
            "SELECT items.id, items.list_id, lists.archived FROM items \
             JOIN lists ON lists.id = items.list_id \
             WHERE items.id = ?1 AND lists.user_id = ?2",
        )
        .bind(&[item_id.into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(check.and_then(|v| {
        let id = v.get("id")?.as_str()?.to_string();
        let list_id = v.get("list_id")?.as_str()?.to_string();
        let list_archived = v
            .get("archived")
            .and_then(|value| value.as_f64())
            .map(|value| value != 0.0)
            .unwrap_or(false);
        Some(OwnedItemState {
            id,
            list_id,
            list_archived,
        })
    }))
}

pub async fn get_owned_item_state_in_list(
    d1: &D1Database,
    item_id: &str,
    list_id: &str,
    user_id: &str,
) -> Result<Option<OwnedItemState>> {
    Ok(get_owned_item_state(d1, item_id, user_id)
        .await?
        .filter(|item| item.list_id == list_id))
}

/// Toggle a boolean field (D1 stores bools as 0/1 floats).
/// Returns `None` if not found, `Some(new_value)` on success.
pub async fn toggle_bool_field(
    d1: &D1Database,
    table: &str,
    column: &str,
    id: &str,
    user_id: &str,
) -> Result<Option<bool>> {
    let row = d1
        .prepare(format!(
            "SELECT {column} FROM {table} WHERE id = ?1 AND user_id = ?2"
        ))
        .bind(&[id.into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let current = row
        .get(column)
        .and_then(|v| v.as_f64())
        .map(|f| f != 0.0)
        .unwrap_or(false);

    let new_val: i32 = if current { 0 } else { 1 };
    d1.prepare(format!(
        "UPDATE {table} SET {column} = ?1, updated_at = datetime('now') WHERE id = ?2"
    ))
    .bind(&[new_val.into(), id.into()])?
    .run()
    .await?;

    Ok(Some(!current))
}

/// Get the next position value (MAX(position) + 1).
pub async fn next_position(
    d1: &D1Database,
    table: &str,
    filter: &str,
    params: &[JsValue],
) -> Result<i32> {
    let max_pos = d1
        .prepare(format!(
            "SELECT COALESCE(MAX(position), -1) as max_pos FROM {table} WHERE {filter}"
        ))
        .bind(params)?
        .first::<serde_json::Value>(None)
        .await?
        .and_then(|v| v.get("max_pos")?.as_i64())
        .unwrap_or(-1);
    Ok((max_pos + 1) as i32)
}

pub async fn apply_positions(d1: &D1Database, table: &str, ids: &[String]) -> Result<()> {
    for (position, id) in ids.iter().enumerate() {
        d1.prepare(format!(
            "UPDATE {table} SET position = ?1, updated_at = datetime('now') WHERE id = ?2"
        ))
        .bind(&[(position as i32).into(), id.clone().into()])?
        .run()
        .await?;
    }

    Ok(())
}

/// Convert `Option<String>` to `JsValue` (Some → string, None → NULL).
pub fn opt_str_to_js(opt: &Option<String>) -> JsValue {
    match opt {
        Some(s) => JsValue::from(s.as_str()),
        None => JsValue::NULL,
    }
}

/// Convert a tri-state nullable string patch into a DB value.
/// None = don't change, Some(None) = set NULL, Some(Some(v)) = set value.
/// When `empty_string_clears` is true, Some(Some("")) also maps to NULL.
pub fn nullable_string_patch_to_js(
    patch: &Option<Option<String>>,
    empty_string_clears: bool,
) -> Option<JsValue> {
    match patch {
        None => None,
        Some(None) => Some(JsValue::NULL),
        Some(Some(value)) if empty_string_clears && value.is_empty() => Some(JsValue::NULL),
        Some(Some(value)) => Some(JsValue::from(value.as_str())),
    }
}

pub fn random_hex_color() -> String {
    let seed = uuid::Uuid::new_v4().simple().to_string();
    format!("#{}", &seed[..6])
}

/// Extract a required route parameter or return an error.
pub fn require_param(ctx: &RouteContext<String>, name: &str) -> Result<String> {
    ctx.param(name)
        .ok_or_else(|| Error::from(format!("Missing {name}")))
        .map(|s| s.to_string())
}

pub async fn parse_json_body<T: DeserializeOwned>(
    req: &mut Request,
) -> std::result::Result<T, Response> {
    let body = req
        .text()
        .await
        .map_err(|_| json_error("invalid_request_body", 400).expect("build 400 response"))?;
    serde_json::from_str::<T>(&body)
        .map_err(|_| json_error("invalid_request_body", 400).expect("build 400 response"))
}

/// Fetch list feature names for a given list_id.
pub async fn get_list_features(d1: &D1Database, list_id: &str) -> Result<Vec<String>> {
    let feature_rows = d1
        .prepare("SELECT feature_name FROM list_features WHERE list_id = ?1")
        .bind(&[list_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;
    Ok(feature_rows
        .iter()
        .filter_map(|r| r.get("feature_name")?.as_str().map(String::from))
        .collect())
}
