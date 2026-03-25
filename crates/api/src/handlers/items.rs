use kartoteka_shared::*;
use wasm_bindgen::JsValue;
use worker::*;

pub async fn list_all(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = ctx.param("list_id").ok_or_else(|| Error::from("Missing list_id"))?.to_string();
    let d1 = ctx.env.d1("DB")?;

    // Verify list belongs to user
    let list_check = d1
        .prepare("SELECT id FROM lists WHERE id = ?1 AND user_id = ?2")
        .bind(&[list_id.clone().into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if list_check.is_none() {
        return Response::error("Not found", 404);
    }

    let result = d1
        .prepare(
            "SELECT id, list_id, title, description, completed, position, quantity, actual_quantity, unit, due_date, due_time, created_at, updated_at \
             FROM items WHERE list_id = ?1 ORDER BY completed ASC, position ASC, created_at ASC",
        )
        .bind(&[list_id.into()])?
        .all()
        .await?;
    let items = result.results::<Item>()?;
    Response::from_json(&items)
}

pub async fn create(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = ctx.param("list_id").ok_or_else(|| Error::from("Missing list_id"))?.to_string();
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
        return Response::error("Not found", 404);
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

    let desc_val: JsValue = match &body.description {
        Some(d) => d.as_str().into(),
        None => JsValue::NULL,
    };

    let quantity_val: JsValue = match body.quantity {
        Some(q) => q.into(),
        None => JsValue::NULL,
    };

    let actual_quantity_val: JsValue = match body.quantity {
        Some(_) => 0i32.into(),
        None => JsValue::NULL,
    };

    let unit_val: JsValue = match &body.unit {
        Some(u) => JsValue::from(u.as_str()),
        None => JsValue::NULL,
    };

    let due_date_val: JsValue = match &body.due_date {
        Some(d) => JsValue::from(d.as_str()),
        None => JsValue::NULL,
    };

    let due_time_val: JsValue = match &body.due_time {
        Some(t) => JsValue::from(t.as_str()),
        None => JsValue::NULL,
    };

    d1.prepare(
        "INSERT INTO items (id, list_id, title, description, position, quantity, actual_quantity, unit, due_date, due_time) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
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
        due_date_val,
        due_time_val,
    ])?
    .run()
    .await?;

    let item = d1
        .prepare(
            "SELECT id, list_id, title, description, completed, position, quantity, actual_quantity, unit, due_date, due_time, created_at, updated_at \
             FROM items WHERE id = ?1",
        )
        .bind(&[id.into()])?
        .first::<Item>(None)
        .await?
        .ok_or_else(|| Error::from("Failed to create item"))?;

    let mut resp = Response::from_json(&item)?;
    resp = resp.with_status(201);
    Ok(resp)
}

pub async fn update(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = ctx.param("id").ok_or_else(|| Error::from("Missing id"))?.to_string();
    let body: UpdateItemRequest = req.json().await?;
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
        return Response::error("Not found", 404);
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
        d1.prepare("UPDATE items SET actual_quantity = ?1, updated_at = datetime('now') WHERE id = ?2")
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
                d1.prepare("UPDATE items SET completed = ?1, updated_at = datetime('now') WHERE id = ?2")
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

    if let Some(due_date) = &body.due_date {
        d1.prepare("UPDATE items SET due_date = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[JsValue::from(due_date.as_str()), id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(due_time) = &body.due_time {
        d1.prepare("UPDATE items SET due_time = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[JsValue::from(due_time.as_str()), id.clone().into()])?
            .run()
            .await?;
    }

    let item = d1
        .prepare(
            "SELECT id, list_id, title, description, completed, position, quantity, actual_quantity, unit, due_date, due_time, created_at, updated_at \
             FROM items WHERE id = ?1",
        )
        .bind(&[id.into()])?
        .first::<Item>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&item)
}

pub async fn delete(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = ctx.param("id").ok_or_else(|| Error::from("Missing id"))?.to_string();
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
        return Response::error("Not found", 404);
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
        return Response::error("Not found", 404);
    }

    // Verify target list belongs to user
    let target_check = d1
        .prepare("SELECT id FROM lists WHERE id = ?1 AND user_id = ?2")
        .bind(&[target_list_id.clone().into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if target_check.is_none() {
        return Response::error("Target list not found", 404);
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

    d1.prepare("UPDATE items SET list_id = ?1, position = ?2, updated_at = datetime('now') WHERE id = ?3")
        .bind(&[target_list_id.into(), position.into(), id.clone().into()])?
        .run()
        .await?;

    let item = d1
        .prepare(
            "SELECT id, list_id, title, description, completed, position, quantity, actual_quantity, unit, due_date, due_time, created_at, updated_at \
             FROM items WHERE id = ?1",
        )
        .bind(&[id.into()])?
        .first::<Item>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&item)
}

/// GET /api/items/by-date?date=YYYY-MM-DD&include_overdue=true
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

    let d1 = ctx.env.d1("DB")?;

    let result = if include_overdue {
        d1.prepare(
            "SELECT i.id, i.list_id, i.title, i.description, i.completed, i.position, \
             i.quantity, i.actual_quantity, i.unit, i.due_date, i.due_time, \
             i.created_at, i.updated_at, l.name as list_name, l.list_type \
             FROM items i \
             JOIN lists l ON l.id = i.list_id \
             WHERE l.user_id = ?1 AND l.archived = 0 \
             AND (i.due_date = ?2 OR (i.due_date < ?2 AND i.completed = 0)) \
             ORDER BY i.completed ASC, i.due_date ASC, l.name ASC, i.due_time ASC, i.position ASC",
        )
        .bind(&[user_id.into(), date.into()])?
        .all()
        .await?
    } else {
        d1.prepare(
            "SELECT i.id, i.list_id, i.title, i.description, i.completed, i.position, \
             i.quantity, i.actual_quantity, i.unit, i.due_date, i.due_time, \
             i.created_at, i.updated_at, l.name as list_name, l.list_type \
             FROM items i \
             JOIN lists l ON l.id = i.list_id \
             WHERE l.user_id = ?1 AND l.archived = 0 AND i.due_date = ?2 \
             ORDER BY i.completed ASC, l.name ASC, i.due_time ASC, i.position ASC",
        )
        .bind(&[user_id.into(), date.into()])?
        .all()
        .await?
    };

    let items = result.results::<DateItem>()?;
    Response::from_json(&items)
}
