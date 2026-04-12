use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::{
    Container, ContainerDetail, CreateContainerRequest, CursorPage, UpdateContainerRequest,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use wasm_bindgen::JsValue;
use worker::*;

use super::CONTAINER_SELECT;

const DEFAULT_LIMIT: u32 = 100;
const MAX_LIMIT: u32 = 100;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ContainersCursorParams {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainersCursorLast {
    pub position: i32,
    pub created_at: String,
    pub id: String,
}

fn query_param(url: &Url, key: &str) -> Option<String> {
    url.query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, value)| value.to_string())
}

fn parse_limit_query(value: Option<String>) -> Result<u32> {
    let Some(value) = value else {
        return Ok(DEFAULT_LIMIT);
    };
    let parsed = value
        .parse::<u32>()
        .map_err(|_| Error::from("invalid limit"))?;
    if parsed == 0 {
        return Err(Error::from("invalid limit"));
    }
    Ok(parsed.min(MAX_LIMIT))
}

pub(crate) async fn list_all_page(
    d1: &D1Database,
    user_id: &str,
    limit: u32,
    cursor: Option<&ContainersCursorLast>,
) -> Result<CursorPage<Container>> {
    let mut sql = format!("{CONTAINER_SELECT} WHERE c.user_id = ?1");
    let mut params: Vec<JsValue> = vec![user_id.into()];

    if let Some(cursor) = cursor {
        sql.push_str(&format!(
            " AND (c.position > ?{} OR (c.position = ?{} AND c.created_at > ?{}) OR (c.position = ?{} AND c.created_at = ?{} AND c.id > ?{}))",
            params.len() + 1,
            params.len() + 2,
            params.len() + 3,
            params.len() + 4,
            params.len() + 5,
            params.len() + 6,
        ));
        params.push(cursor.position.into());
        params.push(cursor.position.into());
        params.push(cursor.created_at.clone().into());
        params.push(cursor.position.into());
        params.push(cursor.created_at.clone().into());
        params.push(cursor.id.clone().into());
    }

    params.push(((limit + 1) as i32).into());
    sql.push_str(&format!(
        " ORDER BY c.position ASC, c.created_at ASC, c.id ASC LIMIT ?{}",
        params.len()
    ));

    let mut items = d1
        .prepare(&sql)
        .bind(&params)?
        .all()
        .await?
        .results::<Container>()?;
    let next_cursor = if items.len() > limit as usize {
        items.truncate(limit as usize);
        items
            .last()
            .map(|last| {
                crate::cursor::encode_cursor(
                    "containers",
                    limit,
                    &ContainersCursorParams::default(),
                    &ContainersCursorLast {
                        position: last.position,
                        created_at: last.created_at.clone(),
                        id: last.id.clone(),
                    },
                )
            })
            .transpose()
            .map_err(|e| Error::from(e.to_string()))?
    } else {
        None
    };

    Ok(CursorPage { items, next_cursor })
}

#[instrument(skip_all)]
pub async fn list_all(req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let limit = parse_limit_query(query_param(&req.url()?, "limit"))?;
    let d1 = ctx.env.d1("DB")?;
    let page = list_all_page(&d1, &user_id, limit, None).await?;
    Response::from_json(&page)
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
