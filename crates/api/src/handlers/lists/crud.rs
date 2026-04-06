use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::*;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use wasm_bindgen::JsValue;
use worker::*;

use super::{LIST_SELECT, create_list_from_request};

const DEFAULT_LIMIT: u32 = 100;
const MAX_LIMIT: u32 = 100;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RootListsCursorParams {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RootListsCursorLast {
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
    cursor: Option<&RootListsCursorLast>,
) -> Result<CursorPage<List>> {
    let mut sql = format!(
        "{LIST_SELECT} WHERE l.user_id = ?1 AND l.parent_list_id IS NULL AND l.container_id IS NULL AND l.archived = 0"
    );
    let mut params: Vec<JsValue> = vec![user_id.into()];

    if let Some(cursor) = cursor {
        sql.push_str(&format!(
            " AND (l.position > ?{} OR (l.position = ?{} AND l.created_at > ?{}) OR (l.position = ?{} AND l.created_at = ?{} AND l.id > ?{}))",
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
        " ORDER BY l.position ASC, l.created_at ASC, l.id ASC LIMIT ?{}",
        params.len()
    ));

    let mut items = d1
        .prepare(&sql)
        .bind(&params)?
        .all()
        .await?
        .results::<List>()?;
    let next_cursor = if items.len() > limit as usize {
        items.truncate(limit as usize);
        items
            .last()
            .map(|last| {
                crate::cursor::encode_cursor(
                    "lists",
                    limit,
                    &RootListsCursorParams::default(),
                    &RootListsCursorLast {
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

#[instrument(skip_all, fields(action = "create_list", list_id = tracing::field::Empty))]
pub async fn create(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let body: CreateListRequest = req.json().await?;
    let d1 = ctx.env.d1("DB")?;
    create_list_from_request(&d1, &user_id, body).await
}

#[instrument(skip_all)]
pub async fn get_one(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    let d1 = ctx.env.d1("DB")?;

    // Track last opened
    let _ = d1
        .prepare("UPDATE lists SET last_opened_at = datetime('now') WHERE id = ?1 AND user_id = ?2")
        .bind(&[id.clone().into(), user_id.clone().into()])?
        .run()
        .await;

    let list = d1
        .prepare(format!("{LIST_SELECT} WHERE l.id = ?1 AND l.user_id = ?2"))
        .bind(&[id.into(), user_id.into()])?
        .first::<List>(None)
        .await?;

    match list {
        Some(l) => Response::from_json(&l),
        None => json_error("list_not_found", 404),
    }
}

#[instrument(skip_all, fields(action = "update_list", list_id = tracing::field::Empty))]
pub async fn update(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("list_id", tracing::field::display(&id));
    let body: UpdateListRequest = match parse_json_body(&mut req).await {
        Ok(body) => body,
        Err(resp) => return Ok(resp),
    };
    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "lists", &id, &user_id).await? {
        return json_error("list_not_found", 404);
    }

    if let Some(name) = &body.name {
        d1.prepare("UPDATE lists SET name = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[name.clone().into(), id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(desc_val) = nullable_string_patch_to_js(&body.description, true) {
        d1.prepare("UPDATE lists SET description = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[desc_val, id.clone().into()])?
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

    if let Some(archived) = body.archived {
        let val: i32 = if archived { 1 } else { 0 };
        d1.prepare("UPDATE lists SET archived = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[val.into(), id.clone().into()])?
            .run()
            .await?;
    }

    let list = d1
        .prepare(format!("{LIST_SELECT} WHERE l.id = ?1 AND l.user_id = ?2"))
        .bind(&[id.into(), user_id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&list)
}

#[instrument(skip_all, fields(action = "delete_list", list_id = tracing::field::Empty))]
pub async fn delete(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("list_id", tracing::field::display(&id));
    let d1 = ctx.env.d1("DB")?;
    d1.prepare("DELETE FROM lists WHERE id = ?1 AND user_id = ?2")
        .bind(&[id.into(), user_id.into()])?
        .run()
        .await?;
    Ok(Response::empty()?.with_status(204))
}

#[instrument(skip_all)]
pub async fn list_sublists(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let parent_id = require_param(&ctx, "id")?;
    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "lists", &parent_id, &user_id).await? {
        return json_error("list_not_found", 404);
    }

    let result = d1
        .prepare(format!(
            "{LIST_SELECT} WHERE l.parent_list_id = ?1 ORDER BY l.position ASC"
        ))
        .bind(&[parent_id.into()])?
        .all()
        .await?;
    let sublists = result.results::<List>()?;
    Response::from_json(&sublists)
}

#[instrument(skip_all, fields(action = "reset_list", list_id = tracing::field::Empty))]
pub async fn reset(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("list_id", tracing::field::display(&id));
    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "lists", &id, &user_id).await? {
        return json_error("list_not_found", 404);
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

#[instrument(skip_all, fields(action = "create_sublist", list_id = tracing::field::Empty))]
pub async fn create_sublist(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let parent_id = require_param(&ctx, "id")?;
    let body: serde_json::Value = req.json().await?;
    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::from("Missing name"))?
        .to_string();
    let create_req = CreateListRequest {
        name,
        list_type: ListType::Custom,
        features: None,
        parent_list_id: Some(parent_id),
        container_id: None,
    };
    let d1 = ctx.env.d1("DB")?;
    create_list_from_request(&d1, &user_id, create_req).await
}
