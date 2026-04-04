use crate::error::{json_error, validation_error};
use crate::helpers::*;
use kartoteka_shared::*;
use tracing::instrument;
use wasm_bindgen::JsValue;
use worker::*;

use super::validation::{
    ItemQuantityState, ItemTemporalState, derive_completed_from_quantity_state, normalize_title,
    validate_item_quantity_state, validate_item_temporal_state,
};
use super::{ITEM_COLS, check_item_features, list_archived_response};

#[instrument(skip_all)]
pub async fn list_all(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "list_id")?;
    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "lists", &list_id, &user_id).await? {
        return json_error("list_not_found", 404);
    }

    let query = format!(
        "SELECT {} FROM items WHERE list_id = ?1 ORDER BY position ASC, created_at ASC",
        ITEM_COLS
    );
    let result = d1.prepare(&query).bind(&[list_id.into()])?.all().await?;
    let items = result.results::<Item>()?;
    Response::from_json(&items)
}

#[instrument(skip_all)]
pub async fn get_one(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "list_id")?;
    let id = require_param(&ctx, "id")?;
    let d1 = ctx.env.d1("DB")?;

    if get_owned_item_state_in_list(&d1, &id, &list_id, &user_id)
        .await?
        .is_none()
    {
        return json_error("item_not_found", 404);
    }

    let query = format!(
        "SELECT {} FROM items WHERE id = ?1 AND list_id = ?2",
        ITEM_COLS
    );
    let item = d1
        .prepare(&query)
        .bind(&[id.into(), list_id.clone().into()])?
        .first::<Item>(None)
        .await?;
    let Some(item) = item else {
        return json_error("item_not_found", 404);
    };

    // Fetch list name + features for the combined response (saves a round-trip from the client)
    #[derive(serde::Deserialize)]
    struct ListRow {
        name: String,
        features: Option<String>,
    }
    let list_row = d1
        .prepare("SELECT name, (SELECT COALESCE(json_group_array(json_object('name', lf.feature_name, 'config', json(lf.config))), '[]') FROM list_features lf WHERE lf.list_id = ?1) as features FROM lists WHERE id = ?1")
        .bind(&[list_id.into()])?
        .first::<ListRow>(None)
        .await?
        .ok_or_else(|| Error::from("List not found"))?;
    let list_features: Vec<ListFeature> = list_row
        .features
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    Response::from_json(&ItemDetailResponse {
        item,
        list_name: list_row.name,
        list_features,
    })
}

#[instrument(skip_all, fields(action = "create_item", item_id = tracing::field::Empty))]
pub async fn create(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "list_id")?;
    let body: CreateItemRequest = match parse_json_body(&mut req).await {
        Ok(body) => body,
        Err(resp) => return Ok(resp),
    };
    let id = uuid::Uuid::new_v4().to_string();
    tracing::Span::current().record("item_id", tracing::field::display(&id));

    let d1 = ctx.env.d1("DB")?;

    let Some(list_state) = get_owned_list_state(&d1, &list_id, &user_id).await? else {
        return json_error("list_not_found", 404);
    };
    if list_state.archived {
        return list_archived_response();
    }

    let position = next_position(&d1, "items", "list_id = ?1", &[list_id.clone().into()]).await?;

    let feature_names = get_list_features(&d1, &list_id).await?;

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

    let mut validation_errors =
        validate_item_temporal_state(&ItemTemporalState::from_create(&body));
    let quantity_state = ItemQuantityState::from_create(&body);
    validate_item_quantity_state(&quantity_state, &mut validation_errors);
    let Some(title) = normalize_title(&body.title, "title", &mut validation_errors) else {
        return validation_error("Invalid item payload.", validation_errors);
    };
    if !validation_errors.is_empty() {
        return validation_error("Invalid item payload.", validation_errors);
    }

    let desc_val = opt_str_to_js(&body.description);
    let quantity_val: JsValue = match body.quantity {
        Some(q) => q.into(),
        None => JsValue::NULL,
    };
    let actual_quantity_val: JsValue = match quantity_state.actual_quantity {
        Some(actual_quantity) => actual_quantity.into(),
        None => JsValue::NULL,
    };
    let completed_val: i32 = if derive_completed_from_quantity_state(&quantity_state) {
        1
    } else {
        0
    };
    let unit_val = opt_str_to_js(&body.unit);
    let start_date_val = opt_str_to_js(&body.start_date);
    let start_time_val = opt_str_to_js(&body.start_time);
    let deadline_val = opt_str_to_js(&body.deadline);
    let deadline_time_val = opt_str_to_js(&body.deadline_time);
    let hard_deadline_val = opt_str_to_js(&body.hard_deadline);

    d1.prepare(
        "INSERT INTO items (id, list_id, title, description, completed, position, quantity, actual_quantity, unit, start_date, start_time, deadline, deadline_time, hard_deadline) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
    )
    .bind(&[
        id.clone().into(),
        list_id.into(),
        title.into(),
        desc_val,
        completed_val.into(),
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

#[instrument(skip_all, fields(action = "update_item", item_id = tracing::field::Empty))]
pub async fn update(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "list_id")?;
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("item_id", tracing::field::display(&id));
    let body: UpdateItemRequest = match parse_json_body(&mut req).await {
        Ok(body) => body,
        Err(resp) => return Ok(resp),
    };
    let d1 = ctx.env.d1("DB")?;

    let item_state = match get_owned_item_state_in_list(&d1, &id, &list_id, &user_id).await? {
        Some(item_state) => item_state,
        None => return json_error("item_not_found", 404),
    };
    if item_state.list_archived {
        return list_archived_response();
    }

    let feature_names = get_list_features(&d1, &item_state.list_id).await?;

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

    let current_item = d1
        .prepare(format!("SELECT {} FROM items WHERE id = ?1", ITEM_COLS))
        .bind(&[id.clone().into()])?
        .first::<Item>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    let mut next_temporal_state = ItemTemporalState::from_item(&current_item);
    next_temporal_state.apply_update(&body);
    let mut next_quantity_state = ItemQuantityState::from_item(&current_item);
    next_quantity_state.apply_update(&body);
    let mut validation_errors = validate_item_temporal_state(&next_temporal_state);
    validate_item_quantity_state(&next_quantity_state, &mut validation_errors);
    let normalized_title = body
        .title
        .as_deref()
        .and_then(|title| normalize_title(title, "title", &mut validation_errors));
    if !validation_errors.is_empty() {
        return validation_error("Invalid item payload.", validation_errors);
    }

    if let Some(title) = normalized_title {
        d1.prepare("UPDATE items SET title = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[title.into(), id.clone().into()])?
            .run()
            .await?;
    }

    if let Some(desc_val) = nullable_string_patch_to_js(&body.description, true) {
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

    let quantity_changed = body.quantity.is_some() || body.actual_quantity.is_some();

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
    }

    if let Some(unit_val) = nullable_string_patch_to_js(&body.unit, false) {
        d1.prepare("UPDATE items SET unit = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[unit_val, id.clone().into()])?
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
        if let Some(js_val) = nullable_string_patch_to_js(field, false) {
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

    if quantity_changed {
        let completed_val: i32 = if derive_completed_from_quantity_state(&next_quantity_state) {
            1
        } else {
            0
        };
        d1.prepare("UPDATE items SET completed = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(&[completed_val.into(), id.clone().into()])?
            .run()
            .await?;
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

#[instrument(skip_all, fields(action = "delete_item", item_id = tracing::field::Empty))]
pub async fn delete(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "list_id")?;
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("item_id", tracing::field::display(&id));
    let d1 = ctx.env.d1("DB")?;

    let item_state = match get_owned_item_state_in_list(&d1, &id, &list_id, &user_id).await? {
        Some(item_state) => item_state,
        None => return json_error("item_not_found", 404),
    };
    if item_state.list_archived {
        return list_archived_response();
    }

    d1.prepare("DELETE FROM items WHERE id = ?1")
        .bind(&[id.into()])?
        .run()
        .await?;
    Ok(Response::empty()?.with_status(204))
}
