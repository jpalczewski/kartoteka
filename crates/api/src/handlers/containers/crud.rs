use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::{
    Container, ContainerDetail, CreateContainerRequest, UpdateContainerRequest,
};
use tracing::instrument;
use worker::*;

use super::CONTAINER_SELECT;

#[instrument(skip_all)]
pub async fn list_all(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let d1 = ctx.env.d1("DB")?;
    let result = d1
        .prepare(format!(
            "{CONTAINER_SELECT} WHERE c.user_id = ?1 ORDER BY c.position ASC, c.created_at ASC"
        ))
        .bind(&[user_id.into()])?
        .all()
        .await?;
    let containers = result.results::<Container>()?;
    Response::from_json(&containers)
}

#[instrument(skip_all, fields(action = "create_container", container_id = tracing::field::Empty))]
pub async fn create(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let body: CreateContainerRequest = req.json().await?;
    let id = uuid::Uuid::new_v4().to_string();
    tracing::Span::current().record("container_id", tracing::field::display(&id));
    let d1 = ctx.env.d1("DB")?;

    if let Some(ref parent_id) = body.parent_container_id {
        if let Some(err) = validate_parent_container(&d1, parent_id, &user_id).await? {
            return Ok(err);
        }
    }

    let status_val = container_status_to_js(&body.status);
    let parent_val = opt_str_to_js(&body.parent_container_id);

    let position = next_position(
        &d1,
        "containers",
        "user_id = ?1 AND parent_container_id IS ?2",
        &[user_id.clone().into(), parent_val.clone()],
    )
    .await?;

    d1.prepare(
        "INSERT INTO containers (id, user_id, name, status, parent_container_id, position) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(&[
        id.clone().into(),
        user_id.clone().into(),
        body.name.clone().into(),
        status_val,
        parent_val,
        position.into(),
    ])?
    .run()
    .await?;

    let container = d1
        .prepare(format!(
            "{CONTAINER_SELECT} WHERE c.id = ?1 AND c.user_id = ?2"
        ))
        .bind(&[id.into(), user_id.into()])?
        .first::<Container>(None)
        .await?
        .ok_or_else(|| Error::from("Failed to create container"))?;

    let mut resp = Response::from_json(&container)?;
    resp = resp.with_status(201);
    Ok(resp)
}

#[instrument(skip_all)]
pub async fn get_one(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    let d1 = ctx.env.d1("DB")?;

    let _ = d1
        .prepare(
            "UPDATE containers SET last_opened_at = datetime('now') WHERE id = ?1 AND user_id = ?2",
        )
        .bind(&[id.clone().into(), user_id.clone().into()])?
        .run()
        .await;

    let container = d1
        .prepare(format!(
            "{CONTAINER_SELECT} WHERE c.id = ?1 AND c.user_id = ?2"
        ))
        .bind(&[id.clone().into(), user_id.into()])?
        .first::<Container>(None)
        .await?;

    let container = match container {
        Some(c) => c,
        None => return json_error("container_not_found", 404),
    };

    let item_progress = d1
        .prepare(
            "SELECT \
             COALESCE(SUM(CASE WHEN i.completed = 1 THEN 1 ELSE 0 END), 0) as completed_items, \
             COUNT(i.id) as total_items \
             FROM items i JOIN lists l ON l.id = i.list_id \
             WHERE l.container_id = ?1",
        )
        .bind(&[id.clone().into()])?
        .first::<serde_json::Value>(None)
        .await?
        .unwrap_or_default();

    let list_progress = d1
        .prepare(
            "SELECT COUNT(*) as total_lists, \
             SUM(CASE WHEN NOT EXISTS (\
               SELECT 1 FROM items i2 WHERE i2.list_id = l.id AND i2.completed = 0\
             ) AND EXISTS (\
               SELECT 1 FROM items i3 WHERE i3.list_id = l.id\
             ) THEN 1 ELSE 0 END) as completed_lists \
             FROM lists l WHERE l.container_id = ?1",
        )
        .bind(&[id.into()])?
        .first::<serde_json::Value>(None)
        .await?
        .unwrap_or_default();

    let completed_items = item_progress
        .get("completed_items")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as u32;
    let total_items = item_progress
        .get("total_items")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as u32;
    let completed_lists = list_progress
        .get("completed_lists")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as u32;
    let total_lists = list_progress
        .get("total_lists")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as u32;

    let detail = ContainerDetail {
        container,
        completed_items,
        total_items,
        completed_lists,
        total_lists,
    };

    Response::from_json(&detail)
}

#[instrument(skip_all, fields(action = "update_container", container_id = tracing::field::Empty))]
pub async fn update(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("container_id", tracing::field::display(&id));
    let body: UpdateContainerRequest = match parse_json_body(&mut req).await {
        Ok(body) => body,
        Err(resp) => return Ok(resp),
    };
    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "containers", &id, &user_id).await? {
        return json_error("container_not_found", 404);
    }

    if let Some(name) = &body.name {
        d1.prepare("UPDATE containers SET name = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[name.clone().into(), id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(desc_val) = nullable_string_patch_to_js(&body.description, true) {
        d1.prepare(
            "UPDATE containers SET description = ?1, updated_at = datetime('now') WHERE id = ?2",
        )
        .bind(&[desc_val, id.clone().into()])?
        .run()
        .await?;
    }

    if let Some(status_opt) = &body.status {
        let status_val = container_status_to_js(status_opt);
        d1.prepare("UPDATE containers SET status = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[status_val, id.clone().into()])?
            .run()
            .await?;
    }

    let container = d1
        .prepare(format!(
            "{CONTAINER_SELECT} WHERE c.id = ?1 AND c.user_id = ?2"
        ))
        .bind(&[id.into(), user_id.into()])?
        .first::<Container>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&container)
}

#[instrument(skip_all, fields(action = "delete_container", container_id = tracing::field::Empty))]
pub async fn delete(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("container_id", tracing::field::display(&id));
    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "containers", &id, &user_id).await? {
        return json_error("container_not_found", 404);
    }

    d1.prepare("DELETE FROM containers WHERE id = ?1")
        .bind(&[id.into()])?
        .run()
        .await?;

    Ok(Response::empty()?.with_status(204))
}
