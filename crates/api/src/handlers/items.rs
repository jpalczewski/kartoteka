use kartoteka_shared::*;
use wasm_bindgen::JsValue;
use worker::*;

pub async fn list_all(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let list_id = ctx.param("list_id").ok_or_else(|| Error::from("Missing list_id"))?;
    let d1 = ctx.env.d1("DB")?;
    let stmt = d1.prepare(
        "SELECT id, list_id, title, description, completed, position, created_at, updated_at \
         FROM items WHERE list_id = ?1 ORDER BY position ASC, created_at ASC",
    );
    let result = stmt.bind(&[list_id.into()])?.all().await?;
    let items = result.results::<Item>()?;
    Response::from_json(&items)
}

pub async fn create(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let list_id = ctx.param("list_id").ok_or_else(|| Error::from("Missing list_id"))?.to_string();
    let body: CreateItemRequest = req.json().await?;
    let id = uuid::Uuid::new_v4().to_string();

    let d1 = ctx.env.d1("DB")?;

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

    d1.prepare(
        "INSERT INTO items (id, list_id, title, description, position) VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(&[
        id.clone().into(),
        list_id.into(),
        body.title.into(),
        desc_val,
        position.into(),
    ])?
    .run()
    .await?;

    let item = d1
        .prepare(
            "SELECT id, list_id, title, description, completed, position, created_at, updated_at \
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
    let id = ctx.param("id").ok_or_else(|| Error::from("Missing id"))?.to_string();
    let body: UpdateItemRequest = req.json().await?;
    let d1 = ctx.env.d1("DB")?;

    if let Some(title) = &body.title {
        d1.prepare("UPDATE items SET title = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[title.clone().into(), id.clone().into()])?
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

    let item = d1
        .prepare(
            "SELECT id, list_id, title, description, completed, position, created_at, updated_at \
             FROM items WHERE id = ?1",
        )
        .bind(&[id.into()])?
        .first::<Item>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&item)
}

pub async fn delete(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let id = ctx.param("id").ok_or_else(|| Error::from("Missing id"))?;
    let d1 = ctx.env.d1("DB")?;
    d1.prepare("DELETE FROM items WHERE id = ?1")
        .bind(&[id.into()])?
        .run()
        .await?;
    Ok(Response::empty()?.with_status(204))
}
