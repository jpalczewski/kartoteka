use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::*;
use tracing::instrument;
use wasm_bindgen::JsValue;
use worker::*;

/// GET /api/tags
#[instrument(skip_all)]
pub async fn list_all(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.as_str();
    let d1 = ctx.env.d1("DB")?;
    let result = d1
        .prepare("SELECT id, user_id, name, color, parent_tag_id, created_at FROM tags WHERE user_id = ?1 ORDER BY name")
        .bind(&[user_id.into()])?
        .all()
        .await?;
    let tags = result.results::<Tag>()?;
    Response::from_json(&tags)
}

/// POST /api/tags
#[instrument(skip_all, fields(action = "create_tag", tag_id = tracing::field::Empty))]
pub async fn create(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let body: CreateTagRequest = req.json().await?;
    let id = uuid::Uuid::new_v4().to_string();
    tracing::Span::current().record("tag_id", &tracing::field::display(&id));

    let parent_val = opt_str_to_js(&body.parent_tag_id);

    let d1 = ctx.env.d1("DB")?;
    d1.prepare(
        "INSERT INTO tags (id, user_id, name, color, parent_tag_id) VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(&[
        id.clone().into(),
        user_id.clone().into(),
        body.name.into(),
        body.color.into(),
        parent_val,
    ])?
    .run()
    .await?;

    let tag = d1
        .prepare(
            "SELECT id, user_id, name, color, parent_tag_id, created_at FROM tags WHERE id = ?1 AND user_id = ?2",
        )
        .bind(&[id.into(), user_id.into()])?
        .first::<Tag>(None)
        .await?
        .ok_or_else(|| Error::from("Failed to create tag"))?;

    Ok(Response::from_json(&tag)?.with_status(201))
}

/// PUT /api/tags/:id
#[instrument(skip_all, fields(action = "update_tag", tag_id = tracing::field::Empty))]
pub async fn update(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("tag_id", &tracing::field::display(&id));
    let body: UpdateTagRequest = req.json().await?;
    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "tags", &id, &user_id).await? {
        return json_error("tag_not_found", 404);
    }

    if let Some(name) = &body.name {
        d1.prepare("UPDATE tags SET name = ?1 WHERE id = ?2")
            .bind(&[name.clone().into(), id.clone().into()])?
            .run()
            .await?;
    }
    if let Some(color) = &body.color {
        d1.prepare("UPDATE tags SET color = ?1 WHERE id = ?2")
            .bind(&[color.clone().into(), id.clone().into()])?
            .run()
            .await?;
    }
    if let Some(parent) = &body.parent_tag_id {
        if let Some(new_parent_id) = parent {
            // Self-reference check first (no DB call needed)
            if new_parent_id == &id {
                return json_error("tag_self_parent", 400);
            }
            // Cycle prevention: check if new parent is a descendant of this tag
            let cycle_check = d1
                .prepare(
                    "WITH RECURSIVE descendants AS ( \
                     SELECT id FROM tags WHERE parent_tag_id = ?1 \
                     UNION ALL \
                     SELECT t.id FROM tags t JOIN descendants d ON t.parent_tag_id = d.id \
                     ) SELECT 1 FROM descendants WHERE id = ?2 LIMIT 1",
                )
                .bind(&[
                    JsValue::from(id.as_str()),
                    JsValue::from(new_parent_id.as_str()),
                ])?
                .first::<serde_json::Value>(None)
                .await?;
            if cycle_check.is_some() {
                return json_error("tag_cycle", 400);
            }
        }

        let parent_val: JsValue = match parent {
            Some(p) => p.as_str().into(),
            None => JsValue::NULL,
        };
        d1.prepare("UPDATE tags SET parent_tag_id = ?1 WHERE id = ?2")
            .bind(&[parent_val, id.clone().into()])?
            .run()
            .await?;
    }

    let tag = d1
        .prepare(
            "SELECT id, user_id, name, color, parent_tag_id, created_at FROM tags WHERE id = ?1 AND user_id = ?2",
        )
        .bind(&[id.into(), user_id.into()])?
        .first::<Tag>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&tag)
}

/// DELETE /api/tags/:id
#[instrument(skip_all, fields(action = "delete_tag", tag_id = tracing::field::Empty))]
pub async fn delete(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("tag_id", &tracing::field::display(&id));
    let d1 = ctx.env.d1("DB")?;
    d1.prepare("DELETE FROM tags WHERE id = ?1 AND user_id = ?2")
        .bind(&[id.into(), user_id.into()])?
        .run()
        .await?;
    Ok(Response::empty()?.with_status(204))
}

/// POST /api/tags/:id/merge
#[instrument(skip_all, fields(action = "merge_tags", tag_id = tracing::field::Empty))]
pub async fn merge(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let source_id = require_param(&ctx, "id")?;
    tracing::Span::current().record("tag_id", &tracing::field::display(&source_id));
    let body: kartoteka_shared::MergeTagRequest = req.json().await?;
    let target_id = body.target_tag_id;
    let d1 = ctx.env.d1("DB")?;

    // Verify both tags belong to user
    if !check_ownership(&d1, "tags", &source_id, &user_id).await? {
        return json_error("tag_not_found", 404);
    }
    if !check_ownership(&d1, "tags", &target_id, &user_id).await? {
        return json_error("tag_not_found", 404);
    }

    // Move item_tags from source to target (skip duplicates)
    d1.prepare("INSERT OR IGNORE INTO item_tags (item_id, tag_id) SELECT item_id, ?1 FROM item_tags WHERE tag_id = ?2")
        .bind(&[target_id.clone().into(), source_id.clone().into()])?
        .run()
        .await?;

    // Move list_tags from source to target (skip duplicates)
    d1.prepare("INSERT OR IGNORE INTO list_tags (list_id, tag_id) SELECT list_id, ?1 FROM list_tags WHERE tag_id = ?2")
        .bind(&[target_id.clone().into(), source_id.clone().into()])?
        .run()
        .await?;

    // Reparent source's children to target
    d1.prepare("UPDATE tags SET parent_tag_id = ?1 WHERE parent_tag_id = ?2")
        .bind(&[target_id.clone().into(), source_id.clone().into()])?
        .run()
        .await?;

    // Delete source tag (cascades remove its item_tags/list_tags)
    d1.prepare("DELETE FROM tags WHERE id = ?1")
        .bind(&[source_id.into()])?
        .run()
        .await?;

    // Return updated target
    let tag = d1
        .prepare(
            "SELECT id, user_id, name, color, parent_tag_id, created_at FROM tags WHERE id = ?1 AND user_id = ?2",
        )
        .bind(&[target_id.into(), user_id.into()])?
        .first::<Tag>(None)
        .await?
        .ok_or_else(|| Error::from("Target not found after merge"))?;

    Response::from_json(&tag)
}

/// POST /api/items/:item_id/tags
#[instrument(skip_all, fields(action = "assign_tag_to_item"))]
pub async fn assign_to_item(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let item_id = require_param(&ctx, "item_id")?;
    let body: TagAssignment = req.json().await?;
    let d1 = ctx.env.d1("DB")?;

    if !check_item_ownership(&d1, &item_id, &user_id).await? {
        return json_error("item_not_found", 404);
    }

    if !check_ownership(&d1, "tags", &body.tag_id, &user_id).await? {
        return json_error("tag_not_found", 404);
    }

    d1.prepare("INSERT OR IGNORE INTO item_tags (item_id, tag_id) VALUES (?1, ?2)")
        .bind(&[item_id.into(), body.tag_id.into()])?
        .run()
        .await?;
    Ok(Response::empty()?.with_status(204))
}

/// DELETE /api/items/:item_id/tags/:tag_id
#[instrument(skip_all, fields(action = "remove_tag_from_item"))]
pub async fn remove_from_item(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let item_id = require_param(&ctx, "item_id")?;
    let tag_id = require_param(&ctx, "tag_id")?;
    let d1 = ctx.env.d1("DB")?;

    if !check_item_ownership(&d1, &item_id, &user_id).await? {
        return json_error("item_not_found", 404);
    }

    d1.prepare("DELETE FROM item_tags WHERE item_id = ?1 AND tag_id = ?2")
        .bind(&[item_id.into(), tag_id.into()])?
        .run()
        .await?;
    Ok(Response::empty()?.with_status(204))
}

/// POST /api/lists/:list_id/tags
#[instrument(skip_all, fields(action = "assign_tag_to_list"))]
pub async fn assign_to_list(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "list_id")?;
    let body: TagAssignment = req.json().await?;
    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "lists", &list_id, &user_id).await? {
        return json_error("list_not_found", 404);
    }

    if !check_ownership(&d1, "tags", &body.tag_id, &user_id).await? {
        return json_error("tag_not_found", 404);
    }

    d1.prepare("INSERT OR IGNORE INTO list_tags (list_id, tag_id) VALUES (?1, ?2)")
        .bind(&[list_id.into(), body.tag_id.into()])?
        .run()
        .await?;
    Ok(Response::empty()?.with_status(204))
}

/// DELETE /api/lists/:list_id/tags/:tag_id
#[instrument(skip_all, fields(action = "remove_tag_from_list"))]
pub async fn remove_from_list(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "list_id")?;
    let tag_id = require_param(&ctx, "tag_id")?;
    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "lists", &list_id, &user_id).await? {
        return json_error("list_not_found", 404);
    }

    d1.prepare("DELETE FROM list_tags WHERE list_id = ?1 AND tag_id = ?2")
        .bind(&[list_id.into(), tag_id.into()])?
        .run()
        .await?;
    Ok(Response::empty()?.with_status(204))
}

/// GET /api/tags/:id/items
#[instrument(skip_all)]
pub async fn tag_items(req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let tag_id = require_param(&ctx, "id")?;
    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "tags", &tag_id, &user_id).await? {
        return json_error("tag_not_found", 404);
    }

    // Check recursive param (default: true)
    let url = req.url()?;
    let recursive = url
        .query_pairs()
        .find(|(k, _)| k == "recursive")
        .map(|(_, v)| v != "false")
        .unwrap_or(true);

    let rows = if recursive {
        d1.prepare(
            "WITH RECURSIVE tag_tree AS ( \
             SELECT id FROM tags WHERE id = ?1 AND user_id = ?2 \
             UNION ALL \
             SELECT t.id FROM tags t JOIN tag_tree tt ON t.parent_tag_id = tt.id WHERE t.user_id = ?2 \
             ) \
             SELECT DISTINCT i.id, i.list_id, i.title, i.description, i.completed, i.position, \
             i.quantity, i.actual_quantity, i.unit, i.start_date, i.start_time, i.deadline, i.deadline_time, i.hard_deadline, \
             i.created_at, i.updated_at, l.name as list_name \
             FROM items i \
             JOIN item_tags it ON it.item_id = i.id \
             JOIN tag_tree tt ON it.tag_id = tt.id \
             JOIN lists l ON l.id = i.list_id \
             ORDER BY l.name, i.position"
        )
        .bind(&[tag_id.into(), user_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?
    } else {
        d1.prepare(
            "SELECT i.id, i.list_id, i.title, i.description, i.completed, i.position, \
             i.quantity, i.actual_quantity, i.unit, i.start_date, i.start_time, i.deadline, i.deadline_time, i.hard_deadline, \
             i.created_at, i.updated_at, l.name as list_name \
             FROM items i \
             JOIN item_tags it ON it.item_id = i.id \
             JOIN lists l ON l.id = i.list_id \
             WHERE it.tag_id = ?1 AND l.user_id = ?2 \
             ORDER BY l.name, i.position",
        )
        .bind(&[tag_id.into(), user_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?
    };

    Response::from_json(&rows)
}

/// GET /api/tag-links/items
#[instrument(skip_all)]
pub async fn all_item_tag_links(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let d1 = ctx.env.d1("DB")?;
    let result = d1
        .prepare("SELECT it.item_id, it.tag_id FROM item_tags it JOIN items i ON i.id = it.item_id JOIN lists l ON l.id = i.list_id JOIN tags t ON t.id = it.tag_id WHERE t.user_id = ?1")
        .bind(&[user_id.into()])?
        .all()
        .await?;
    let links = result.results::<ItemTagLink>()?;
    Response::from_json(&links)
}

/// GET /api/tag-links/lists
#[instrument(skip_all)]
pub async fn all_list_tag_links(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let d1 = ctx.env.d1("DB")?;
    let result = d1
        .prepare("SELECT lt.list_id, lt.tag_id FROM list_tags lt JOIN tags t ON t.id = lt.tag_id WHERE t.user_id = ?1")
        .bind(&[user_id.into()])?
        .all()
        .await?;
    let links = result.results::<ListTagLink>()?;
    Response::from_json(&links)
}
