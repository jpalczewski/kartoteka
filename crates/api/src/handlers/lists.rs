use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::*;
use tracing::instrument;
use worker::*;

pub const LIST_SELECT: &str = "\
    SELECT l.id, l.user_id, l.name, l.description, l.list_type, \
    l.parent_list_id, l.position, l.archived, l.container_id, l.pinned, l.last_opened_at, \
    l.created_at, l.updated_at, \
    COALESCE((SELECT json_group_array(json_object('name', lf.feature_name, 'config', json(lf.config))) \
    FROM list_features lf WHERE lf.list_id = l.id), '[]') as features \
    FROM lists l";

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
    let id = uuid::Uuid::new_v4().to_string();
    tracing::Span::current().record("list_id", tracing::field::display(&id));
    let list_type_str = serde_json::to_value(&body.list_type)
        .map_err(|e| Error::from(e.to_string()))?
        .as_str()
        .unwrap_or("custom")
        .to_string();

    let d1 = ctx.env.d1("DB")?;
    d1.prepare("INSERT INTO lists (id, user_id, name, list_type) VALUES (?1, ?2, ?3, ?4)")
        .bind(&[
            id.clone().into(),
            user_id.clone().into(),
            body.name.clone().into(),
            list_type_str.into(),
        ])?
        .run()
        .await?;

    // Insert features (from request or defaults from ListType)
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

    let mut resp = Response::from_json(&list)?;
    resp = resp.with_status(201);
    Ok(resp)
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
    let body: UpdateListRequest = req.json().await?;
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
    let id = uuid::Uuid::new_v4().to_string();
    tracing::Span::current().record("list_id", tracing::field::display(&id));
    let d1 = ctx.env.d1("DB")?;

    // Verify parent belongs to user and is a top-level list
    let parent = d1
        .prepare("SELECT id FROM lists WHERE id = ?1 AND user_id = ?2 AND parent_list_id IS NULL")
        .bind(&[parent_id.clone().into(), user_id.clone().into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if parent.is_none() {
        return json_error("list_not_found", 404);
    }

    let position = next_position(
        &d1,
        "lists",
        "parent_list_id = ?1",
        &[parent_id.clone().into()],
    )
    .await?;

    d1.prepare(
        "INSERT INTO lists (id, user_id, name, list_type, parent_list_id, position) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(&[
        id.clone().into(),
        user_id.clone().into(),
        name.into(),
        "custom".into(),
        parent_id.into(),
        position.into(),
    ])?
    .run()
    .await?;

    let sublist = d1
        .prepare(format!("{LIST_SELECT} WHERE l.id = ?1 AND l.user_id = ?2"))
        .bind(&[id.into(), user_id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Failed to create sublist"))?;

    let mut resp = Response::from_json(&sublist)?;
    resp = resp.with_status(201);
    Ok(resp)
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

    if !check_ownership(&d1, "lists", &id, &user_id).await? {
        return json_error("list_not_found", 404);
    }

    // If target container specified, verify ownership
    if let Some(ref cid) = body.container_id {
        if !check_ownership(&d1, "containers", cid, &user_id).await? {
            return json_error("container_not_found", 404);
        }
    }

    let cid_val = opt_str_to_js(&body.container_id);

    d1.prepare("UPDATE lists SET container_id = ?1, updated_at = datetime('now') WHERE id = ?2")
        .bind(&[cid_val, id.clone().into()])?
        .run()
        .await?;

    let list = d1
        .prepare(format!("{LIST_SELECT} WHERE l.id = ?1 AND l.user_id = ?2"))
        .bind(&[id.into(), user_id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&list)
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
