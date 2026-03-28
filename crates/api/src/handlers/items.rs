use crate::error::json_error;
use kartoteka_shared::*;
use wasm_bindgen::JsValue;
use worker::*;

/// Common SELECT columns for Item struct
const ITEM_COLS: &str = "id, list_id, title, description, completed, position, quantity, actual_quantity, unit, start_date, start_time, deadline, deadline_time, hard_deadline, created_at, updated_at";

/// Common SELECT columns for DateItem struct (with list info)
const DATE_ITEM_COLS: &str = "i.id, i.list_id, i.title, i.description, i.completed, i.position, \
    i.quantity, i.actual_quantity, i.unit, i.start_date, i.start_time, i.deadline, i.deadline_time, i.hard_deadline, \
    i.created_at, i.updated_at, l.name as list_name, l.list_type";

pub async fn list_all(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = ctx
        .param("list_id")
        .ok_or_else(|| Error::from("Missing list_id"))?
        .to_string();
    let d1 = ctx.env.d1("DB")?;

    // Verify list belongs to user
    let list_check = d1
        .prepare("SELECT id FROM lists WHERE id = ?1 AND user_id = ?2")
        .bind(&[list_id.clone().into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if list_check.is_none() {
        return json_error("list_not_found", 404);
    }

    let query = format!(
        "SELECT {} FROM items WHERE list_id = ?1 ORDER BY completed ASC, position ASC, created_at ASC",
        ITEM_COLS
    );
    let result = d1.prepare(&query).bind(&[list_id.into()])?.all().await?;
    let items = result.results::<Item>()?;
    Response::from_json(&items)
}

pub async fn create(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = ctx
        .param("list_id")
        .ok_or_else(|| Error::from("Missing list_id"))?
        .to_string();
    let body: CreateItemRequest = req.json().await?;
    let id = uuid::Uuid::new_v4().to_string();

    let d1 = ctx.env.d1("DB")?;

    // Verify list belongs to user
    let list_check = d1
        .prepare("SELECT id FROM lists WHERE id = ?1 AND user_id = ?2")
        .bind(&[list_id.clone().into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if list_check.is_none() {
        return json_error("list_not_found", 404);
    }

    // Get next position
    let max_pos = d1
        .prepare("SELECT COALESCE(MAX(position), -1) as max_pos FROM items WHERE list_id = ?1")
        .bind(&[list_id.clone().into()])?
        .first::<serde_json::Value>(None)
        .await?
        .and_then(|v| v.get("max_pos")?.as_i64())
        .unwrap_or(-1);
    let position = (max_pos + 1) as i32;

    let feature_rows = d1
        .prepare("SELECT feature_name FROM list_features WHERE list_id = ?1")
        .bind(&[list_id.clone().into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;
    let feature_names: Vec<String> = feature_rows
        .iter()
        .filter_map(|r| r.get("feature_name")?.as_str().map(String::from))
        .collect();

    let has_date_field = body.start_date.is_some()
        || body.deadline.is_some()
        || body.hard_deadline.is_some()
        || body.start_time.is_some()
        || body.deadline_time.is_some();
    let has_quantity_field = body.quantity.is_some() || body.unit.is_some();

    if let Some(err_resp) = check_item_features(&feature_names, has_date_field, has_quantity_field)?
    {
        return Ok(err_resp);
    }

    let opt_str = |v: &Option<String>| -> JsValue {
        match v {
            Some(s) => JsValue::from(s.as_str()),
            None => JsValue::NULL,
        }
    };

    let desc_val = opt_str(&body.description);
    let quantity_val: JsValue = match body.quantity {
        Some(q) => q.into(),
        None => JsValue::NULL,
    };
    let actual_quantity_val: JsValue = match body.quantity {
        Some(_) => 0i32.into(),
        None => JsValue::NULL,
    };
    let unit_val = opt_str(&body.unit);
    let start_date_val = opt_str(&body.start_date);
    let start_time_val = opt_str(&body.start_time);
    let deadline_val = opt_str(&body.deadline);
    let deadline_time_val = opt_str(&body.deadline_time);
    let hard_deadline_val = opt_str(&body.hard_deadline);

    d1.prepare(
        "INSERT INTO items (id, list_id, title, description, position, quantity, actual_quantity, unit, start_date, start_time, deadline, deadline_time, hard_deadline) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
    )
    .bind(&[
        id.clone().into(),
        list_id.into(),
        body.title.into(),
        desc_val,
        position.into(),
        quantity_val,
        actual_quantity_val,
        unit_val,
        start_date_val,
        start_time_val,
        deadline_val,
        deadline_time_val,
        hard_deadline_val,
    ])?
    .run()
    .await?;

    let select_query = format!("SELECT {} FROM items WHERE id = ?1", ITEM_COLS);
    let item = d1
        .prepare(&select_query)
        .bind(&[id.into()])?
        .first::<Item>(None)
        .await?
        .ok_or_else(|| Error::from("Failed to create item"))?;

    let mut resp = Response::from_json(&item)?;
    resp = resp.with_status(201);
    Ok(resp)
}

/// Helper to handle Option<Option<String>> date fields in update:
/// None = don't change, Some(None) = set NULL, Some(Some(v)) = set value
fn update_nullable_str(outer: &Option<Option<String>>) -> Option<JsValue> {
    match outer {
        None => None, // don't change
        Some(None) => Some(JsValue::NULL),
        Some(Some(s)) => Some(JsValue::from(s.as_str())),
    }
}

fn check_item_features(
    feature_names: &[String],
    has_date_field: bool,
    has_quantity_field: bool,
) -> worker::Result<Option<Response>> {
    if has_date_field && !feature_names.iter().any(|f| f == FEATURE_DEADLINES) {
        return Ok(Some(Response::error(
            r#"{"error":"feature_required","feature":"deadlines","message":"This list does not have the 'deadlines' feature enabled. Enable it in list settings or retry without date fields."}"#,
            422,
        )?));
    }
    if has_quantity_field && !feature_names.iter().any(|f| f == FEATURE_QUANTITY) {
        return Ok(Some(Response::error(
            r#"{"error":"feature_required","feature":"quantity","message":"This list does not have the 'quantity' feature enabled. Enable it in list settings or retry without quantity fields."}"#,
            422,
        )?));
    }
    Ok(None)
}

pub async fn update(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = ctx
        .param("id")
        .ok_or_else(|| Error::from("Missing id"))?
        .to_string();
    let body: UpdateItemRequest = req.json().await?;
    let d1 = ctx.env.d1("DB")?;

    // Verify item belongs to user (via list ownership)
    let item_check = d1
        .prepare(
            "SELECT items.id, items.list_id FROM items \
             JOIN lists ON lists.id = items.list_id \
             WHERE items.id = ?1 AND lists.user_id = ?2",
        )
        .bind(&[id.clone().into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if item_check.is_none() {
        return json_error("item_not_found", 404);
    }

    let list_id_for_features = item_check
        .as_ref()
        .and_then(|v| v.get("list_id")?.as_str().map(String::from))
        .ok_or_else(|| Error::from("Missing list_id on item"))?;

    let feature_rows = d1
        .prepare("SELECT feature_name FROM list_features WHERE list_id = ?1")
        .bind(&[list_id_for_features.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;
    let feature_names: Vec<String> = feature_rows
        .iter()
        .filter_map(|r| r.get("feature_name")?.as_str().map(String::from))
        .collect();

    let has_date_field = matches!(&body.start_date, Some(Some(_)))
        || matches!(&body.deadline, Some(Some(_)))
        || matches!(&body.hard_deadline, Some(Some(_)))
        || matches!(&body.start_time, Some(Some(_)))
        || matches!(&body.deadline_time, Some(Some(_)));
    let has_quantity_field =
        body.quantity.is_some() || body.actual_quantity.is_some() || body.unit.is_some();

    if let Some(err_resp) = check_item_features(&feature_names, has_date_field, has_quantity_field)?
    {
        return Ok(err_resp);
    }

    if let Some(title) = &body.title {
        d1.prepare("UPDATE items SET title = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[title.clone().into(), id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(description) = &body.description {
        let desc_val: JsValue = description.as_str().into();
        d1.prepare("UPDATE items SET description = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[desc_val, id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(completed) = body.completed {
        let val: i32 = if completed { 1 } else { 0 };
        d1.prepare("UPDATE items SET completed = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[val.into(), id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(position) = body.position {
        d1.prepare("UPDATE items SET position = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[position.into(), id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(quantity) = body.quantity {
        d1.prepare("UPDATE items SET quantity = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[JsValue::from(quantity), id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(actual) = body.actual_quantity {
        d1.prepare(
            "UPDATE items SET actual_quantity = ?1, updated_at = datetime('now') WHERE id = ?2",
        )
        .bind(&[JsValue::from(actual), id.clone().into()])?
        .run()
        .await?;

        // Auto-complete: check if actual >= target
        let row = d1
            .prepare("SELECT quantity FROM items WHERE id = ?1")
            .bind(&[id.clone().into()])?
            .first::<serde_json::Value>(None)
            .await?;
        if let Some(row) = row {
            if let Some(target) = row.get("quantity").and_then(|v| v.as_i64()) {
                let completed_val: i32 = if (actual as i64) >= target { 1 } else { 0 };
                d1.prepare(
                    "UPDATE items SET completed = ?1, updated_at = datetime('now') WHERE id = ?2",
                )
                .bind(&[JsValue::from(completed_val), id.clone().into()])?
                .run()
                .await?;
            }
        }
    }

    if let Some(unit) = &body.unit {
        d1.prepare("UPDATE items SET unit = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[JsValue::from(unit.as_str()), id.clone().into()])?
            .run()
            .await?;
    }

    // Date fields: Option<Option<String>> — None=skip, Some(None)=NULL, Some(Some(v))=set
    let date_updates: [(&str, &Option<Option<String>>); 5] = [
        ("start_date", &body.start_date),
        ("start_time", &body.start_time),
        ("deadline", &body.deadline),
        ("deadline_time", &body.deadline_time),
        ("hard_deadline", &body.hard_deadline),
    ];

    for (col, field) in &date_updates {
        if let Some(js_val) = update_nullable_str(field) {
            let sql = format!(
                "UPDATE items SET {} = ?1, updated_at = datetime('now') WHERE id = ?2",
                col
            );
            d1.prepare(&sql)
                .bind(&[js_val, id.clone().into()])?
                .run()
                .await?;
        }
    }

    let select_query = format!("SELECT {} FROM items WHERE id = ?1", ITEM_COLS);
    let item = d1
        .prepare(&select_query)
        .bind(&[id.into()])?
        .first::<Item>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&item)
}

pub async fn delete(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = ctx
        .param("id")
        .ok_or_else(|| Error::from("Missing id"))?
        .to_string();
    let d1 = ctx.env.d1("DB")?;

    // Verify item belongs to user (via list ownership)
    let item_check = d1
        .prepare(
            "SELECT items.id FROM items \
             JOIN lists ON lists.id = items.list_id \
             WHERE items.id = ?1 AND lists.user_id = ?2",
        )
        .bind(&[id.clone().into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if item_check.is_none() {
        return json_error("item_not_found", 404);
    }

    d1.prepare("DELETE FROM items WHERE id = ?1")
        .bind(&[id.into()])?
        .run()
        .await?;
    Ok(Response::empty()?.with_status(204))
}

pub async fn move_item(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = ctx
        .param("id")
        .ok_or_else(|| Error::from("Missing id"))?
        .to_string();
    let body: serde_json::Value = req.json().await?;
    let target_list_id = body
        .get("target_list_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::from("Missing target_list_id"))?
        .to_string();
    let d1 = ctx.env.d1("DB")?;

    // Verify item belongs to user
    let item_check = d1
        .prepare(
            "SELECT items.id FROM items \
             JOIN lists ON lists.id = items.list_id \
             WHERE items.id = ?1 AND lists.user_id = ?2",
        )
        .bind(&[id.clone().into(), user_id.clone().into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if item_check.is_none() {
        return json_error("item_not_found", 404);
    }

    // Verify target list belongs to user
    let target_check = d1
        .prepare("SELECT id FROM lists WHERE id = ?1 AND user_id = ?2")
        .bind(&[target_list_id.clone().into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if target_check.is_none() {
        return json_error("list_not_found", 404);
    }

    // Get next position in target list
    let max_pos = d1
        .prepare("SELECT COALESCE(MAX(position), -1) as max_pos FROM items WHERE list_id = ?1")
        .bind(&[target_list_id.clone().into()])?
        .first::<serde_json::Value>(None)
        .await?
        .and_then(|v| v.get("max_pos")?.as_i64())
        .unwrap_or(-1);
    let position = (max_pos + 1) as i32;

    d1.prepare(
        "UPDATE items SET list_id = ?1, position = ?2, updated_at = datetime('now') WHERE id = ?3",
    )
    .bind(&[target_list_id.into(), position.into(), id.clone().into()])?
    .run()
    .await?;

    let select_query = format!("SELECT {} FROM items WHERE id = ?1", ITEM_COLS);
    let item = d1
        .prepare(&select_query)
        .bind(&[id.into()])?
        .first::<Item>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&item)
}

/// GET /api/items/by-date?date=YYYY-MM-DD&date_field=deadline&include_overdue=true
pub async fn by_date(req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let url = req.url()?;

    let date = url
        .query_pairs()
        .find(|(k, _)| k == "date")
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| Error::from("Missing date parameter"))?;

    let include_overdue = url
        .query_pairs()
        .find(|(k, _)| k == "include_overdue")
        .map(|(_, v)| v != "false")
        .unwrap_or(true);

    let date_field = url
        .query_pairs()
        .find(|(k, _)| k == "date_field")
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| "deadline".to_string());

    let d1 = ctx.env.d1("DB")?;

    if date_field == "all" {
        // UNION ALL across all three date fields
        let sql = format!(
            "SELECT {cols}, 'start' as date_type \
             FROM items i JOIN lists l ON l.id = i.list_id \
             WHERE l.user_id = ?1 AND l.archived = 0 AND i.start_date = ?2 \
             UNION ALL \
             SELECT {cols}, 'deadline' as date_type \
             FROM items i JOIN lists l ON l.id = i.list_id \
             WHERE l.user_id = ?1 AND l.archived = 0 \
             AND (i.deadline = ?2{overdue}) \
             UNION ALL \
             SELECT {cols}, 'hard_deadline' as date_type \
             FROM items i JOIN lists l ON l.id = i.list_id \
             WHERE l.user_id = ?1 AND l.archived = 0 AND i.hard_deadline = ?2 \
             ORDER BY completed ASC, list_name ASC, deadline_time ASC, position ASC",
            cols = DATE_ITEM_COLS,
            overdue = if include_overdue {
                " OR (i.deadline < ?2 AND i.completed = 0)"
            } else {
                ""
            },
        );
        let result = d1
            .prepare(&sql)
            .bind(&[user_id.into(), date.into()])?
            .all()
            .await?;
        let items = result.results::<DateItem>()?;
        Response::from_json(&items)
    } else {
        // Single date field query
        let col = match date_field.as_str() {
            "start_date" => "i.start_date",
            "hard_deadline" => "i.hard_deadline",
            _ => "i.deadline",
        };

        let sql = if include_overdue {
            format!(
                "SELECT {cols}, NULL as date_type \
                 FROM items i JOIN lists l ON l.id = i.list_id \
                 WHERE l.user_id = ?1 AND l.archived = 0 \
                 AND ({col} = ?2 OR ({col} < ?2 AND i.completed = 0)) \
                 ORDER BY i.completed ASC, {col} ASC, l.name ASC, i.deadline_time ASC, i.position ASC",
                cols = DATE_ITEM_COLS,
                col = col,
            )
        } else {
            format!(
                "SELECT {cols}, NULL as date_type \
                 FROM items i JOIN lists l ON l.id = i.list_id \
                 WHERE l.user_id = ?1 AND l.archived = 0 AND {col} = ?2 \
                 ORDER BY i.completed ASC, l.name ASC, i.deadline_time ASC, i.position ASC",
                cols = DATE_ITEM_COLS,
                col = col,
            )
        };

        let result = d1
            .prepare(&sql)
            .bind(&[user_id.into(), date.into()])?
            .all()
            .await?;
        let items = result.results::<DateItem>()?;
        Response::from_json(&items)
    }
}

/// GET /api/items/calendar?from=YYYY-MM-DD&to=YYYY-MM-DD&date_field=deadline&detail=counts|full
pub async fn calendar(req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let url = req.url()?;

    let from = url
        .query_pairs()
        .find(|(k, _)| k == "from")
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| Error::from("Missing from parameter"))?;

    let to = url
        .query_pairs()
        .find(|(k, _)| k == "to")
        .map(|(_, v)| v.to_string())
        .ok_or_else(|| Error::from("Missing to parameter"))?;

    let detail = url
        .query_pairs()
        .find(|(k, _)| k == "detail")
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| "counts".to_string());

    let date_field = url
        .query_pairs()
        .find(|(k, _)| k == "date_field")
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| "deadline".to_string());

    let d1 = ctx.env.d1("DB")?;

    if detail == "full" {
        if date_field == "all" {
            let sql = format!(
                "SELECT {cols}, 'start' as date_type \
                 FROM items i JOIN lists l ON l.id = i.list_id \
                 WHERE l.user_id = ?1 AND l.archived = 0 AND i.start_date >= ?2 AND i.start_date <= ?3 \
                 UNION ALL \
                 SELECT {cols}, 'deadline' as date_type \
                 FROM items i JOIN lists l ON l.id = i.list_id \
                 WHERE l.user_id = ?1 AND l.archived = 0 AND i.deadline >= ?2 AND i.deadline <= ?3 \
                 UNION ALL \
                 SELECT {cols}, 'hard_deadline' as date_type \
                 FROM items i JOIN lists l ON l.id = i.list_id \
                 WHERE l.user_id = ?1 AND l.archived = 0 AND i.hard_deadline >= ?2 AND i.hard_deadline <= ?3 \
                 ORDER BY completed ASC, list_name ASC, deadline_time ASC, position ASC",
                cols = DATE_ITEM_COLS,
            );
            let result = d1
                .prepare(&sql)
                .bind(&[user_id.into(), from.into(), to.into()])?
                .all()
                .await?;
            let items = result.results::<DateItem>()?;

            // Group by the date relevant to date_type
            let mut day_map: std::collections::BTreeMap<String, Vec<DateItem>> =
                std::collections::BTreeMap::new();
            for item in items {
                let date = match item.date_type.as_deref() {
                    Some("start") => item.start_date.clone().unwrap_or_default(),
                    Some("hard_deadline") => item.hard_deadline.clone().unwrap_or_default(),
                    _ => item.deadline.clone().unwrap_or_default(),
                };
                day_map.entry(date).or_default().push(item);
            }
            let day_items: Vec<DayItems> = day_map
                .into_iter()
                .map(|(date, items)| DayItems { date, items })
                .collect();

            Response::from_json(&day_items)
        } else {
            let col = match date_field.as_str() {
                "start_date" => "i.start_date",
                "hard_deadline" => "i.hard_deadline",
                _ => "i.deadline",
            };
            let sql = format!(
                "SELECT {cols}, NULL as date_type \
                 FROM items i JOIN lists l ON l.id = i.list_id \
                 WHERE l.user_id = ?1 AND l.archived = 0 \
                 AND {col} >= ?2 AND {col} <= ?3 \
                 ORDER BY {col} ASC, i.completed ASC, l.name ASC, i.deadline_time ASC, i.position ASC",
                cols = DATE_ITEM_COLS,
                col = col,
            );
            let result = d1
                .prepare(&sql)
                .bind(&[user_id.into(), from.into(), to.into()])?
                .all()
                .await?;
            let items = result.results::<DateItem>()?;

            let mut day_map: std::collections::BTreeMap<String, Vec<DateItem>> =
                std::collections::BTreeMap::new();
            for item in items {
                let date = match date_field.as_str() {
                    "start_date" => item.start_date.clone().unwrap_or_default(),
                    "hard_deadline" => item.hard_deadline.clone().unwrap_or_default(),
                    _ => item.deadline.clone().unwrap_or_default(),
                };
                day_map.entry(date).or_default().push(item);
            }
            let day_items: Vec<DayItems> = day_map
                .into_iter()
                .map(|(date, items)| DayItems { date, items })
                .collect();

            Response::from_json(&day_items)
        }
    } else {
        // Counts mode
        if date_field == "all" {
            let sql = "SELECT date, COUNT(DISTINCT id) as total, \
                 CAST(SUM(CASE WHEN completed = 1 THEN 1 ELSE 0 END) AS INTEGER) as completed \
                 FROM ( \
                     SELECT i.id, i.start_date as date, i.completed FROM items i JOIN lists l ON l.id = i.list_id \
                     WHERE l.user_id = ?1 AND l.archived = 0 AND i.start_date >= ?2 AND i.start_date <= ?3 \
                     UNION ALL \
                     SELECT i.id, i.deadline as date, i.completed FROM items i JOIN lists l ON l.id = i.list_id \
                     WHERE l.user_id = ?1 AND l.archived = 0 AND i.deadline >= ?2 AND i.deadline <= ?3 \
                     UNION ALL \
                     SELECT i.id, i.hard_deadline as date, i.completed FROM items i JOIN lists l ON l.id = i.list_id \
                     WHERE l.user_id = ?1 AND l.archived = 0 AND i.hard_deadline >= ?2 AND i.hard_deadline <= ?3 \
                 ) GROUP BY date ORDER BY date ASC";
            let result = d1
                .prepare(sql)
                .bind(&[user_id.into(), from.into(), to.into()])?
                .all()
                .await?;
            let summaries = result.results::<DaySummary>()?;
            Response::from_json(&summaries)
        } else {
            let col = match date_field.as_str() {
                "start_date" => "i.start_date",
                "hard_deadline" => "i.hard_deadline",
                _ => "i.deadline",
            };
            let sql = format!(
                "SELECT {col} as date, \
                 COUNT(*) as total, \
                 CAST(SUM(i.completed) AS INTEGER) as completed \
                 FROM items i \
                 JOIN lists l ON l.id = i.list_id \
                 WHERE l.user_id = ?1 AND l.archived = 0 \
                 AND {col} >= ?2 AND {col} <= ?3 \
                 GROUP BY {col} \
                 ORDER BY {col} ASC",
                col = col,
            );
            let result = d1
                .prepare(&sql)
                .bind(&[user_id.into(), from.into(), to.into()])?
                .all()
                .await?;
            let summaries = result.results::<DaySummary>()?;
            Response::from_json(&summaries)
        }
    }
}
