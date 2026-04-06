use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::*;
use tracing::instrument;
use worker::*;

use super::{ITEM_COLS, list_archived_response};

async fn fetch_item_by_id(d1: &D1Database, item_id: &str) -> Result<Item> {
    let select_query = format!("SELECT {} FROM items WHERE id = ?1", ITEM_COLS);
    d1.prepare(&select_query)
        .bind(&[item_id.into()])?
        .first::<Item>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))
}

async fn move_items_to_list(
    d1: &D1Database,
    user_id: &str,
    body: MoveItemsRequest,
) -> Result<std::result::Result<Vec<Item>, Response>> {
    if let Err(code) = body.validate() {
        return json_error(code, 400).map(Err);
    }

    let target_list_id = body.target_list_id;
    let item_ids = dedupe_ids(&body.item_ids);

    let Some(target_list_state) = get_owned_list_state(d1, &target_list_id, user_id).await? else {
        return json_error("list_not_found", 404).map(Err);
    };
    if target_list_state.archived {
        return list_archived_response().map(Err);
    }

    let base_position = next_position(
        d1,
        "items",
        "list_id = ?1",
        &[target_list_id.clone().into()],
    )
    .await?;
    let mut moved = Vec::with_capacity(item_ids.len());

    for item_id in &item_ids {
        let source_item_state = match get_owned_item_state(d1, item_id, user_id).await? {
            Some(item_state) => item_state,
            None => return json_error("item_not_found", 404).map(Err),
        };
        if source_item_state.list_archived {
            return list_archived_response().map(Err);
        }
    }

    for (offset, item_id) in item_ids.iter().enumerate() {
        d1.prepare(
            "UPDATE items SET list_id = ?1, position = ?2, updated_at = datetime('now') WHERE id = ?3",
        )
        .bind(&[
            target_list_id.clone().into(),
            (base_position + offset as i32).into(),
            item_id.clone().into(),
        ])?
        .run()
        .await?;

        moved.push(fetch_item_by_id(d1, item_id).await?);
    }

    Ok(Ok(moved))
}

#[instrument(skip_all, fields(action = "move_item", item_id = tracing::field::Empty))]
pub async fn move_item(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("item_id", tracing::field::display(id.as_str()));
    #[derive(serde::Deserialize)]
    struct SingleMoveItemRequest {
        target_list_id: String,
    }
    let body: SingleMoveItemRequest = match parse_json_body(&mut req).await {
        Ok(body) => body,
        Err(resp) => return Ok(resp),
    };
    let d1 = ctx.env.d1("DB")?;
    let item = match move_items_to_list(
        &d1,
        &user_id,
        MoveItemsRequest {
            item_ids: vec![id],
            target_list_id: body.target_list_id,
        },
    )
    .await?
    {
        Ok(mut items) => items.pop().ok_or_else(|| Error::from("Not found"))?,
        Err(resp) => return Ok(resp),
    };

    Response::from_json(&item)
}

#[instrument(skip_all, fields(action = "move_items", item_count = tracing::field::Empty, target_list_id = tracing::field::Empty))]
pub async fn move_batch(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let body: MoveItemsRequest = match parse_json_body(&mut req).await {
        Ok(body) => body,
        Err(resp) => return Ok(resp),
    };
    tracing::Span::current().record("item_count", body.item_ids.len());
    tracing::Span::current().record(
        "target_list_id",
        tracing::field::display(body.target_list_id.as_str()),
    );

    let d1 = ctx.env.d1("DB")?;
    match move_items_to_list(&d1, &user_id, body).await? {
        Ok(items) => Response::from_json(&items),
        Err(resp) => Ok(resp),
    }
}

#[instrument(
    skip_all,
    fields(
        action = "set_item_placement",
        item_id = tracing::field::Empty,
        source_list_id = tracing::field::Empty,
        target_list_id = tracing::field::Empty
    )
)]
pub async fn set_placement(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("item_id", tracing::field::display(id.as_str()));

    let body: SetItemPlacementRequest = match parse_json_body(&mut req).await {
        Ok(body) => body,
        Err(resp) => return Ok(resp),
    };
    tracing::Span::current().record(
        "source_list_id",
        tracing::field::display(body.source_list_id.as_str()),
    );
    tracing::Span::current().record(
        "target_list_id",
        tracing::field::display(body.target_list_id.as_str()),
    );

    if let Err(code) = body.validate() {
        return json_error(code, 400);
    }

    let d1 = ctx.env.d1("DB")?;

    let source_item_state = match get_owned_item_state(&d1, &id, &user_id).await? {
        Some(item_state) => item_state,
        None => return json_error("item_not_found", 404),
    };
    if source_item_state.list_archived {
        return list_archived_response();
    }
    if source_item_state.list_id != body.source_list_id {
        return json_error("invalid_item_placement", 400);
    }

    let Some(source_list_state) = get_owned_list_state(&d1, &body.source_list_id, &user_id).await?
    else {
        return json_error("list_not_found", 404);
    };
    if source_list_state.archived {
        return list_archived_response();
    }

    let Some(target_list_state) = get_owned_list_state(&d1, &body.target_list_id, &user_id).await?
    else {
        return json_error("list_not_found", 404);
    };
    if target_list_state.archived {
        return list_archived_response();
    }

    let current_source_ids = fetch_ordered_ids(
        &d1,
        "SELECT id FROM items WHERE list_id = ?1 ORDER BY position ASC, created_at ASC",
        &[body.source_list_id.clone().into()],
    )
    .await?;
    let current_target_ids = fetch_ordered_ids(
        &d1,
        "SELECT id FROM items WHERE list_id = ?1 ORDER BY position ASC, created_at ASC",
        &[body.target_list_id.clone().into()],
    )
    .await?;

    let expected_source_ids: Vec<String> = current_source_ids
        .iter()
        .filter(|current_id| current_id.as_str() != id.as_str())
        .cloned()
        .collect();
    if !ids_match_exact_set(&expected_source_ids, &body.source_item_ids) {
        return json_error("invalid_item_placement", 400);
    }

    let mut expected_target_ids = current_target_ids.clone();
    expected_target_ids.push(id.clone());
    if !ids_match_exact_set(&expected_target_ids, &body.target_item_ids) {
        return json_error("invalid_item_placement", 400);
    }

    d1.prepare("UPDATE items SET list_id = ?1, updated_at = datetime('now') WHERE id = ?2")
        .bind(&[body.target_list_id.clone().into(), id.clone().into()])?
        .run()
        .await?;

    apply_positions(&d1, "items", &body.source_item_ids).await?;
    apply_positions(&d1, "items", &body.target_item_ids).await?;

    let select_query = format!("SELECT {} FROM items WHERE id = ?1", ITEM_COLS);
    let item = d1
        .prepare(&select_query)
        .bind(&[id.into()])?
        .first::<Item>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&item)
}

#[instrument(skip_all, fields(action = "reorder_items", list_id = tracing::field::Empty, item_count = tracing::field::Empty))]
pub async fn reorder(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "list_id")?;
    tracing::Span::current().record("list_id", tracing::field::display(&list_id));
    let body: ReorderItemsRequest = req.json().await?;
    tracing::Span::current().record("item_count", body.item_ids.len());

    if let Err(code) = body.validate() {
        return json_error(code, 400);
    }

    let d1 = ctx.env.d1("DB")?;

    let Some(list_state) = get_owned_list_state(&d1, &list_id, &user_id).await? else {
        return json_error("list_not_found", 404);
    };
    if list_state.archived {
        return list_archived_response();
    }

    let current_ids = fetch_ordered_ids(
        &d1,
        "SELECT id FROM items WHERE list_id = ?1 ORDER BY position ASC, created_at ASC",
        &[list_id.clone().into()],
    )
    .await?;
    if !ids_match_exact_set(&current_ids, &body.item_ids) {
        return json_error("invalid_item_reorder", 400);
    }

    apply_positions(&d1, "items", &body.item_ids).await?;
    Ok(Response::empty()?.with_status(204))
}
