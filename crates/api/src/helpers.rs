use wasm_bindgen::JsValue;
use worker::*;

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

/// Check item ownership and return its list_id. Returns `None` if not owned.
pub async fn check_item_ownership_with_list(
    d1: &D1Database,
    item_id: &str,
    user_id: &str,
) -> Result<Option<String>> {
    let check = d1
        .prepare(
            "SELECT items.id, items.list_id FROM items \
             JOIN lists ON lists.id = items.list_id \
             WHERE items.id = ?1 AND lists.user_id = ?2",
        )
        .bind(&[item_id.into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(check.and_then(|v| v.get("list_id")?.as_str().map(String::from)))
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

/// Convert `Option<String>` to `JsValue` (Some → string, None → NULL).
pub fn opt_str_to_js(opt: &Option<String>) -> JsValue {
    match opt {
        Some(s) => JsValue::from(s.as_str()),
        None => JsValue::NULL,
    }
}

/// Extract a required route parameter or return an error.
pub fn require_param(ctx: &RouteContext<String>, name: &str) -> Result<String> {
    ctx.param(name)
        .ok_or_else(|| Error::from(format!("Missing {name}")))
        .map(|s| s.to_string())
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
