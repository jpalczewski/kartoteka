use kartoteka_shared::*;
use worker::*;

pub async fn list_all(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let d1 = ctx.env.d1("DB")?;
    let stmt = d1.prepare("SELECT id, name, list_type, created_at, updated_at FROM lists ORDER BY updated_at DESC");
    let result = stmt.all().await?;
    let lists = result.results::<List>()?;
    Response::from_json(&lists)
}

pub async fn create(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let body: CreateListRequest = req.json().await?;
    let id = uuid::Uuid::new_v4().to_string();
    let list_type_str = serde_json::to_value(&body.list_type)
        .map_err(|e| Error::from(e.to_string()))?
        .as_str()
        .unwrap_or("custom")
        .to_string();

    let d1 = ctx.env.d1("DB")?;
    let stmt = d1.prepare("INSERT INTO lists (id, name, list_type) VALUES (?1, ?2, ?3)");
    stmt.bind(&[id.clone().into(), body.name.clone().into(), list_type_str.into()])?
        .run()
        .await?;

    let fetch_stmt = d1.prepare("SELECT id, name, list_type, created_at, updated_at FROM lists WHERE id = ?1");
    let list = fetch_stmt
        .bind(&[id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Failed to create list"))?;

    let mut resp = Response::from_json(&list)?;
    resp = resp.with_status(201);
    Ok(resp)
}

pub async fn get_one(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let id = ctx.param("id").ok_or_else(|| Error::from("Missing id"))?;
    let d1 = ctx.env.d1("DB")?;
    let stmt = d1.prepare("SELECT id, name, list_type, created_at, updated_at FROM lists WHERE id = ?1");
    let list = stmt
        .bind(&[id.into()])?
        .first::<List>(None)
        .await?;

    match list {
        Some(l) => Response::from_json(&l),
        None => Response::error("Not found", 404),
    }
}

pub async fn update(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let id = ctx.param("id").ok_or_else(|| Error::from("Missing id"))?.to_string();
    let body: UpdateListRequest = req.json().await?;
    let d1 = ctx.env.d1("DB")?;

    if let Some(name) = &body.name {
        d1.prepare("UPDATE lists SET name = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[name.clone().into(), id.clone().into()])?
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

    let list = d1
        .prepare("SELECT id, name, list_type, created_at, updated_at FROM lists WHERE id = ?1")
        .bind(&[id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&list)
}

pub async fn delete(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let id = ctx.param("id").ok_or_else(|| Error::from("Missing id"))?;
    let d1 = ctx.env.d1("DB")?;
    d1.prepare("DELETE FROM lists WHERE id = ?1")
        .bind(&[id.into()])?
        .run()
        .await?;
    Ok(Response::empty()?.with_status(204))
}
