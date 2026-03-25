use kartoteka_shared::*;
use worker::*;

pub async fn list_all(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let d1 = ctx.env.d1("DB")?;
    let result = d1
        .prepare(
            "SELECT id, user_id, name, description, list_type, parent_list_id, position, archived, has_quantity, has_due_date, created_at, updated_at \
             FROM lists WHERE user_id = ?1 AND parent_list_id IS NULL AND archived = 0 ORDER BY updated_at DESC",
        )
        .bind(&[user_id.into()])?
        .all()
        .await?;
    let lists = result.results::<List>()?;
    Response::from_json(&lists)
}

pub async fn create(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let body: CreateListRequest = req.json().await?;
    let id = uuid::Uuid::new_v4().to_string();
    let list_type_str = serde_json::to_value(&body.list_type)
        .map_err(|e| Error::from(e.to_string()))?
        .as_str()
        .unwrap_or("custom")
        .to_string();

    let has_quantity: i32 = if body.has_quantity { 1 } else { 0 };
    let has_due_date: i32 = if body.has_due_date { 1 } else { 0 };

    let d1 = ctx.env.d1("DB")?;
    d1.prepare("INSERT INTO lists (id, user_id, name, list_type, has_quantity, has_due_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
        .bind(&[
            id.clone().into(),
            user_id.clone().into(),
            body.name.clone().into(),
            list_type_str.into(),
            has_quantity.into(),
            has_due_date.into(),
        ])?
        .run()
        .await?;

    let list = d1
        .prepare(
            "SELECT id, user_id, name, description, list_type, parent_list_id, position, archived, has_quantity, has_due_date, created_at, updated_at \
             FROM lists WHERE id = ?1",
        )
        .bind(&[id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Failed to create list"))?;

    let mut resp = Response::from_json(&list)?;
    resp = resp.with_status(201);
    Ok(resp)
}

pub async fn get_one(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = ctx.param("id").ok_or_else(|| Error::from("Missing id"))?.to_string();
    let d1 = ctx.env.d1("DB")?;
    let list = d1
        .prepare(
            "SELECT id, user_id, name, description, list_type, parent_list_id, position, archived, has_quantity, has_due_date, created_at, updated_at \
             FROM lists WHERE id = ?1 AND user_id = ?2",
        )
        .bind(&[id.into(), user_id.into()])?
        .first::<List>(None)
        .await?;

    match list {
        Some(l) => Response::from_json(&l),
        None => Response::error("Not found", 404),
    }
}

pub async fn update(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = ctx.param("id").ok_or_else(|| Error::from("Missing id"))?.to_string();
    let body: UpdateListRequest = req.json().await?;
    let d1 = ctx.env.d1("DB")?;

    // Verify ownership first
    let existing = d1
        .prepare("SELECT id FROM lists WHERE id = ?1 AND user_id = ?2")
        .bind(&[id.clone().into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if existing.is_none() {
        return Response::error("Not found", 404);
    }

    if let Some(name) = &body.name {
        d1.prepare("UPDATE lists SET name = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[name.clone().into(), id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(description) = &body.description {
        d1.prepare("UPDATE lists SET description = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[description.clone().into(), id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(list_type) = &body.list_type {
        let lt = serde_json::to_value(list_type)
            .map_err(|e| Error::from(e.to_string()))?
            .as_str()
            .unwrap_or("custom")
            .to_string();
        d1.prepare("UPDATE lists SET list_type = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[lt.into(), id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(has_quantity) = body.has_quantity {
        let val: i32 = if has_quantity { 1 } else { 0 };
        d1.prepare("UPDATE lists SET has_quantity = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[val.into(), id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(has_due_date) = body.has_due_date {
        let val: i32 = if has_due_date { 1 } else { 0 };
        d1.prepare("UPDATE lists SET has_due_date = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[val.into(), id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(archived) = body.archived {
        let val: i32 = if archived { 1 } else { 0 };
        d1.prepare("UPDATE lists SET archived = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[val.into(), id.clone().into()])?
            .run()
            .await?;
    }

    let list = d1
        .prepare(
            "SELECT id, user_id, name, description, list_type, parent_list_id, position, archived, has_quantity, has_due_date, created_at, updated_at \
             FROM lists WHERE id = ?1",
        )
        .bind(&[id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&list)
}

pub async fn delete(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = ctx.param("id").ok_or_else(|| Error::from("Missing id"))?.to_string();
    let d1 = ctx.env.d1("DB")?;
    d1.prepare("DELETE FROM lists WHERE id = ?1 AND user_id = ?2")
        .bind(&[id.into(), user_id.into()])?
        .run()
        .await?;
    Ok(Response::empty()?.with_status(204))
}

pub async fn list_sublists(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let parent_id = ctx
        .param("id")
        .ok_or_else(|| Error::from("Missing id"))?
        .to_string();
    let d1 = ctx.env.d1("DB")?;

    // Verify parent belongs to user
    let parent = d1
        .prepare("SELECT id FROM lists WHERE id = ?1 AND user_id = ?2")
        .bind(&[parent_id.clone().into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if parent.is_none() {
        return Response::error("Not found", 404);
    }

    let result = d1
        .prepare(
            "SELECT id, user_id, name, description, list_type, parent_list_id, position, archived, has_quantity, has_due_date, created_at, updated_at \
             FROM lists WHERE parent_list_id = ?1 ORDER BY position ASC",
        )
        .bind(&[parent_id.into()])?
        .all()
        .await?;
    let sublists = result.results::<List>()?;
    Response::from_json(&sublists)
}

pub async fn list_archived(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let d1 = ctx.env.d1("DB")?;
    let result = d1
        .prepare(
            "SELECT id, user_id, name, description, list_type, parent_list_id, position, archived, has_quantity, has_due_date, created_at, updated_at \
             FROM lists WHERE user_id = ?1 AND parent_list_id IS NULL AND archived = 1 ORDER BY updated_at DESC",
        )
        .bind(&[user_id.into()])?
        .all()
        .await?;
    let lists = result.results::<List>()?;
    Response::from_json(&lists)
}

pub async fn toggle_archive(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = ctx
        .param("id")
        .ok_or_else(|| Error::from("Missing id"))?
        .to_string();
    let d1 = ctx.env.d1("DB")?;

    // Get current archived state
    let row = d1
        .prepare("SELECT archived FROM lists WHERE id = ?1 AND user_id = ?2")
        .bind(&[id.clone().into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    let current = row
        .as_ref()
        .and_then(|r| r.get("archived"))
        .and_then(|v| v.as_f64())
        .map(|f| f != 0.0)
        .unwrap_or(false);

    let new_val: i32 = if current { 0 } else { 1 };
    d1.prepare("UPDATE lists SET archived = ?1, updated_at = datetime('now') WHERE id = ?2")
        .bind(&[new_val.into(), id.clone().into()])?
        .run()
        .await?;

    let list = d1
        .prepare(
            "SELECT id, user_id, name, description, list_type, parent_list_id, position, archived, has_quantity, has_due_date, created_at, updated_at \
             FROM lists WHERE id = ?1",
        )
        .bind(&[id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&list)
}

pub async fn reset(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = ctx
        .param("id")
        .ok_or_else(|| Error::from("Missing id"))?
        .to_string();
    let d1 = ctx.env.d1("DB")?;

    // Verify ownership
    let check = d1
        .prepare("SELECT id FROM lists WHERE id = ?1 AND user_id = ?2")
        .bind(&[id.clone().into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if check.is_none() {
        return Response::error("Not found", 404);
    }

    // Reset items in main list
    d1.prepare(
        "UPDATE items SET completed = 0, actual_quantity = 0, updated_at = datetime('now') WHERE list_id = ?1",
    )
    .bind(&[id.clone().into()])?
    .run()
    .await?;

    // Reset items in all sublists
    d1.prepare(
        "UPDATE items SET completed = 0, actual_quantity = 0, updated_at = datetime('now') \
         WHERE list_id IN (SELECT id FROM lists WHERE parent_list_id = ?1)",
    )
    .bind(&[id.into()])?
    .run()
    .await?;

    Ok(Response::empty()?.with_status(204))
}

pub async fn create_sublist(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let parent_id = ctx
        .param("id")
        .ok_or_else(|| Error::from("Missing id"))?
        .to_string();
    let body: serde_json::Value = req.json().await?;
    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::from("Missing name"))?
        .to_string();
    let id = uuid::Uuid::new_v4().to_string();
    let d1 = ctx.env.d1("DB")?;

    // Verify parent belongs to user and is a top-level list
    let parent = d1
        .prepare("SELECT id FROM lists WHERE id = ?1 AND user_id = ?2 AND parent_list_id IS NULL")
        .bind(&[parent_id.clone().into(), user_id.clone().into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if parent.is_none() {
        return Response::error("Not found", 404);
    }

    // Get next position
    let max_pos = d1
        .prepare("SELECT COALESCE(MAX(position), -1) as max_pos FROM lists WHERE parent_list_id = ?1")
        .bind(&[parent_id.clone().into()])?
        .first::<serde_json::Value>(None)
        .await?
        .and_then(|v| v.get("max_pos")?.as_i64())
        .unwrap_or(-1);
    let position = (max_pos + 1) as i32;

    d1.prepare(
        "INSERT INTO lists (id, user_id, name, list_type, parent_list_id, position) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(&[
        id.clone().into(),
        user_id.into(),
        name.into(),
        "custom".into(),
        parent_id.into(),
        position.into(),
    ])?
    .run()
    .await?;

    let sublist = d1
        .prepare(
            "SELECT id, user_id, name, description, list_type, parent_list_id, position, archived, has_quantity, has_due_date, created_at, updated_at \
             FROM lists WHERE id = ?1",
        )
        .bind(&[id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Failed to create sublist"))?;

    let mut resp = Response::from_json(&sublist)?;
    resp = resp.with_status(201);
    Ok(resp)
}
