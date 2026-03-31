use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::*;
use tracing::instrument;
use wasm_bindgen::JsValue;
use worker::*;

pub const LIST_SELECT: &str = "\
    SELECT l.id, l.user_id, l.name, l.description, l.list_type, \
    l.parent_list_id, l.position, l.archived, l.container_id, l.pinned, l.last_opened_at, \
    l.created_at, l.updated_at, \
    COALESCE((SELECT json_group_array(json_object('name', lf.feature_name, 'config', json(lf.config))) \
    FROM list_features lf WHERE lf.list_id = l.id), '[]') as features \
    FROM lists l";

fn placement_filter(
    parent_list_id: Option<&str>,
    container_id: Option<&str>,
) -> (&'static str, Vec<JsValue>) {
    match (parent_list_id, container_id) {
        (Some(parent_id), None) => ("parent_list_id = ?1", vec![parent_id.into()]),
        (None, Some(container_id)) => (
            "parent_list_id IS NULL AND container_id = ?1",
            vec![container_id.into()],
        ),
        (None, None) => ("parent_list_id IS NULL AND container_id IS NULL", vec![]),
        (Some(_), Some(_)) => unreachable!("validated earlier"),
    }
}

async fn ensure_parent_list_target(
    d1: &D1Database,
    parent_id: &str,
    user_id: &str,
) -> Result<bool> {
    Ok(d1
        .prepare("SELECT id FROM lists WHERE id = ?1 AND user_id = ?2 AND parent_list_id IS NULL")
        .bind(&[parent_id.into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?
        .is_some())
}

async fn list_has_sublists(d1: &D1Database, list_id: &str) -> Result<bool> {
    Ok(d1
        .prepare("SELECT 1 FROM lists WHERE parent_list_id = ?1 LIMIT 1")
        .bind(&[list_id.into()])?
        .first::<serde_json::Value>(None)
        .await?
        .is_some())
}

async fn touch_list_updated_at(d1: &D1Database, list_id: &str) -> Result<()> {
    d1.prepare("UPDATE lists SET updated_at = datetime('now') WHERE id = ?1")
        .bind(&[list_id.into()])?
        .run()
        .await?;
    Ok(())
}

async fn fetch_lists_by_ids(
    d1: &D1Database,
    user_id: &str,
    list_ids: &[String],
) -> Result<Vec<List>> {
    let mut lists = Vec::with_capacity(list_ids.len());
    for list_id in list_ids {
        let list = d1
            .prepare(format!("{LIST_SELECT} WHERE l.id = ?1 AND l.user_id = ?2"))
            .bind(&[list_id.clone().into(), user_id.into()])?
            .first::<List>(None)
            .await?
            .ok_or_else(|| Error::from("Not found"))?;
        lists.push(list);
    }
    Ok(lists)
}

async fn apply_list_placement(
    d1: &D1Database,
    user_id: &str,
    list_ids: &[String],
    parent_list_id: Option<String>,
    container_id: Option<String>,
) -> Result<Vec<List>> {
    let deduped_ids = dedupe_ids(list_ids);
    if deduped_ids.is_empty() {
        return Err(Error::from("list_ids must not be empty"));
    }

    for list_id in &deduped_ids {
        if !check_ownership(d1, "lists", list_id, user_id).await? {
            return Err(Error::from("list_not_found"));
        }
    }

    if let Some(ref parent_id) = parent_list_id {
        if deduped_ids.iter().any(|list_id| list_id == parent_id) {
            return Err(Error::from("list_self_parent"));
        }
        if !ensure_parent_list_target(d1, parent_id, user_id).await? {
            return Err(Error::from("list_not_found"));
        }
        for list_id in &deduped_ids {
            if list_has_sublists(d1, list_id).await? {
                return Err(Error::from("list_has_sublists"));
            }
        }
    }

    if let Some(ref target_container_id) = container_id
        && !check_ownership(d1, "containers", target_container_id, user_id).await?
    {
        return Err(Error::from("container_not_found"));
    }

    let (filter, params) = placement_filter(parent_list_id.as_deref(), container_id.as_deref());
    let position = next_position(d1, "lists", filter, &params).await?;
    let parent_val = opt_str_to_js(&parent_list_id);
    let container_val = opt_str_to_js(&container_id);

    for (index, list_id) in deduped_ids.iter().enumerate() {
        let next_pos = position + index as i32;
        d1.prepare(
            "UPDATE lists SET parent_list_id = ?1, container_id = ?2, position = ?3, updated_at = datetime('now') WHERE id = ?4",
        )
        .bind(&[
            parent_val.clone(),
            container_val.clone(),
            next_pos.into(),
            list_id.clone().into(),
        ])?
        .run()
        .await?;
    }

    fetch_lists_by_ids(d1, user_id, &deduped_ids).await
}

async fn create_list_from_request(
    d1: &D1Database,
    user_id: &str,
    body: CreateListRequest,
) -> Result<Response> {
    if let Err(code) = body.validate_placement() {
        return json_error(code, 400);
    }

    if let Some(ref parent_id) = body.parent_list_id
        && !ensure_parent_list_target(d1, parent_id, user_id).await?
    {
        return json_error("list_not_found", 404);
    }

    if let Some(ref container_id) = body.container_id
        && !check_ownership(d1, "containers", container_id, user_id).await?
    {
        return json_error("container_not_found", 404);
    }

    let id = uuid::Uuid::new_v4().to_string();
    tracing::Span::current().record("list_id", tracing::field::display(&id));
    let list_type_str = serde_json::to_value(&body.list_type)
        .map_err(|e| Error::from(e.to_string()))?
        .as_str()
        .unwrap_or("custom")
        .to_string();
    let (filter, params) =
        placement_filter(body.parent_list_id.as_deref(), body.container_id.as_deref());
    let position = next_position(d1, "lists", filter, &params).await?;
    let parent_val = opt_str_to_js(&body.parent_list_id);
    let container_val = opt_str_to_js(&body.container_id);

    d1.prepare(
        "INSERT INTO lists (id, user_id, name, list_type, parent_list_id, container_id, position) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(&[
        id.clone().into(),
        user_id.into(),
        body.name.clone().into(),
        list_type_str.into(),
        parent_val,
        container_val,
        position.into(),
    ])?
    .run()
    .await?;

    let features = body
        .features
        .unwrap_or_else(|| body.list_type.default_features());
    for feature in &features {
        let config_str = feature.config.to_string();
        d1.prepare("INSERT INTO list_features (list_id, feature_name, config) VALUES (?1, ?2, ?3)")
            .bind(&[
                id.clone().into(),
                feature.name.clone().into(),
                config_str.into(),
            ])?
            .run()
            .await?;
    }

    let list = d1
        .prepare(format!("{LIST_SELECT} WHERE l.id = ?1 AND l.user_id = ?2"))
        .bind(&[id.into(), user_id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Failed to create list"))?;

    Ok(Response::from_json(&list)?.with_status(201))
}

#[instrument(skip_all)]
pub async fn list_all(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let d1 = ctx.env.d1("DB")?;
    let result = d1
        .prepare(format!(
            "{LIST_SELECT} WHERE l.user_id = ?1 AND l.parent_list_id IS NULL AND l.container_id IS NULL AND l.archived = 0 ORDER BY l.updated_at DESC"
        ))
        .bind(&[user_id.into()])?
        .all()
        .await?;
    let lists = result.results::<List>()?;
    Response::from_json(&lists)
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

#[instrument(skip_all)]
pub async fn list_archived(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let d1 = ctx.env.d1("DB")?;
    let result = d1
        .prepare(format!(
            "{LIST_SELECT} WHERE l.user_id = ?1 AND l.parent_list_id IS NULL AND l.archived = 1 ORDER BY l.updated_at DESC"
        ))
        .bind(&[user_id.into()])?
        .all()
        .await?;
    let lists = result.results::<List>()?;
    Response::from_json(&lists)
}

#[instrument(skip_all, fields(action = "toggle_archive", list_id = tracing::field::Empty))]
pub async fn toggle_archive(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("list_id", tracing::field::display(&id));
    let d1 = ctx.env.d1("DB")?;

    if toggle_bool_field(&d1, "lists", "archived", &id, &user_id)
        .await?
        .is_none()
    {
        return json_error("list_not_found", 404);
    }

    let list = d1
        .prepare(format!("{LIST_SELECT} WHERE l.id = ?1 AND l.user_id = ?2"))
        .bind(&[id.into(), user_id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&list)
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

// === Feature CRUD ===

#[instrument(skip_all, fields(action = "add_list_feature", list_id = tracing::field::Empty))]
pub async fn add_feature(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "id")?;
    tracing::Span::current().record("list_id", tracing::field::display(&list_id));
    let feature_name = require_param(&ctx, "name")?;

    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "lists", &list_id, &user_id).await? {
        return json_error("list_not_found", 404);
    }

    // Parse config from body (default to {})
    let body: FeatureConfigRequest = req.json().await.unwrap_or(FeatureConfigRequest {
        config: serde_json::json!({}),
    });

    // Validate config is a valid JSON object
    if !body.config.is_object() && !body.config.is_null() {
        return json_error("invalid_config", 400);
    }

    let config_str = body.config.to_string();

    d1.prepare(
        "INSERT OR REPLACE INTO list_features (list_id, feature_name, config) VALUES (?1, ?2, ?3)",
    )
    .bind(&[
        list_id.clone().into(),
        feature_name.into(),
        config_str.into(),
    ])?
    .run()
    .await?;
    touch_list_updated_at(&d1, &list_id).await?;

    // Return updated list
    let list = d1
        .prepare(format!("{LIST_SELECT} WHERE l.id = ?1 AND l.user_id = ?2"))
        .bind(&[list_id.into(), user_id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&list)
}

#[instrument(skip_all, fields(action = "remove_list_feature", list_id = tracing::field::Empty))]
pub async fn remove_feature(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "id")?;
    tracing::Span::current().record("list_id", tracing::field::display(&list_id));
    let feature_name = require_param(&ctx, "name")?;

    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "lists", &list_id, &user_id).await? {
        return json_error("list_not_found", 404);
    }

    d1.prepare("DELETE FROM list_features WHERE list_id = ?1 AND feature_name = ?2")
        .bind(&[list_id.clone().into(), feature_name.into()])?
        .run()
        .await?;
    touch_list_updated_at(&d1, &list_id).await?;

    // Return updated list
    let list = d1
        .prepare(format!("{LIST_SELECT} WHERE l.id = ?1 AND l.user_id = ?2"))
        .bind(&[list_id.into(), user_id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&list)
}

// === Container assignment ===

#[instrument(skip_all, fields(action = "move_list", list_id = tracing::field::Empty))]
pub async fn move_list(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("list_id", tracing::field::display(&id));
    let body: MoveListRequest = req.json().await?;
    let d1 = ctx.env.d1("DB")?;
    match apply_list_placement(&d1, &user_id, &[id], None, body.container_id).await {
        Ok(mut lists) => Response::from_json(&lists.remove(0)),
        Err(err) if err.to_string() == "list_not_found" => json_error("list_not_found", 404),
        Err(err) if err.to_string() == "container_not_found" => {
            json_error("container_not_found", 404)
        }
        Err(err) => json_error(err.to_string().as_str(), 400),
    }
}

#[instrument(
    skip_all,
    fields(
        action = "set_list_placement",
        list_count = tracing::field::Empty,
        target_kind = tracing::field::Empty
    )
)]
pub async fn set_placement(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let body: SetListPlacementRequest = req.json().await?;
    tracing::Span::current().record("list_count", body.list_ids.len());
    tracing::Span::current().record(
        "target_kind",
        tracing::field::display(if body.parent_list_id.is_some() {
            "parent_list"
        } else if body.container_id.is_some() {
            "container"
        } else {
            "root"
        }),
    );
    if let Err(code) = body.validate() {
        return json_error(code, 400);
    }
    let d1 = ctx.env.d1("DB")?;

    match apply_list_placement(
        &d1,
        &user_id,
        &body.list_ids,
        body.parent_list_id,
        body.container_id,
    )
    .await
    {
        Ok(moved_lists) => Response::from_json(&serde_json::json!({
            "moved_lists": moved_lists,
        })),
        Err(err) if err.to_string() == "list_not_found" => json_error("list_not_found", 404),
        Err(err) if err.to_string() == "container_not_found" => {
            json_error("container_not_found", 404)
        }
        Err(err) => json_error(err.to_string().as_str(), 400),
    }
}

#[instrument(skip_all, fields(action = "toggle_list_pin", list_id = tracing::field::Empty))]
pub async fn toggle_pin(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("list_id", tracing::field::display(&id));
    let d1 = ctx.env.d1("DB")?;

    if toggle_bool_field(&d1, "lists", "pinned", &id, &user_id)
        .await?
        .is_none()
    {
        return json_error("list_not_found", 404);
    }

    let list = d1
        .prepare(format!("{LIST_SELECT} WHERE l.id = ?1 AND l.user_id = ?2"))
        .bind(&[id.into(), user_id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&list)
}
