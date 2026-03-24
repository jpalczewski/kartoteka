use kartoteka_shared::*;
use wasm_bindgen::JsValue;
use worker::*;

/// GET /api/tags
pub async fn list_all(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.as_str();
    let d1 = ctx.env.d1("DB")?;
    let result = d1
        .prepare("SELECT id, user_id, name, color, category, parent_tag_id, created_at FROM tags WHERE user_id = ?1 ORDER BY category, name")
        .bind(&[user_id.into()])?
        .all()
        .await?;
    let tags = result.results::<Tag>()?;
    Response::from_json(&tags)
}

/// POST /api/tags
pub async fn create(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let body: CreateTagRequest = req.json().await?;
    let id = uuid::Uuid::new_v4().to_string();
    let category_str = serde_json::to_value(&body.category)
        .map_err(|e| Error::from(e.to_string()))?
        .as_str()
        .unwrap_or("custom")
        .to_string();

    let parent_val: JsValue = match &body.parent_tag_id {
        Some(p) => p.as_str().into(),
        None => JsValue::NULL,
    };

    let d1 = ctx.env.d1("DB")?;
    d1.prepare("INSERT INTO tags (id, user_id, name, color, category, parent_tag_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")
        .bind(&[id.clone().into(), user_id.into(), body.name.into(), body.color.into(), category_str.into(), parent_val])?
        .run()
        .await?;

    let tag = d1
        .prepare("SELECT id, user_id, name, color, category, parent_tag_id, created_at FROM tags WHERE id = ?1")
        .bind(&[id.into()])?
        .first::<Tag>(None)
        .await?
        .ok_or_else(|| Error::from("Failed to create tag"))?;

    Ok(Response::from_json(&tag)?.with_status(201))
}

/// PUT /api/tags/:id
pub async fn update(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = ctx
        .param("id")
        .ok_or_else(|| Error::from("Missing id"))?
        .to_string();
    let body: UpdateTagRequest = req.json().await?;
    let d1 = ctx.env.d1("DB")?;

    // Verify ownership
    let existing = d1
        .prepare("SELECT id FROM tags WHERE id = ?1 AND user_id = ?2")
        .bind(&[id.clone().into(), user_id.clone().into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if existing.is_none() {
        return Response::error("Not found", 404);
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
    if let Some(category) = &body.category {
        let cat_str = serde_json::to_value(category)
            .map_err(|e| Error::from(e.to_string()))?
            .as_str()
            .unwrap_or("custom")
            .to_string();
        d1.prepare("UPDATE tags SET category = ?1 WHERE id = ?2")
            .bind(&[cat_str.into(), id.clone().into()])?
            .run()
            .await?;
    }
    if let Some(parent) = &body.parent_tag_id {
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
        .prepare("SELECT id, user_id, name, color, category, parent_tag_id, created_at FROM tags WHERE id = ?1")
        .bind(&[id.into()])?
        .first::<Tag>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&tag)
}

/// DELETE /api/tags/:id
pub async fn delete(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = ctx
        .param("id")
        .ok_or_else(|| Error::from("Missing id"))?
        .to_string();
    let d1 = ctx.env.d1("DB")?;
    d1.prepare("DELETE FROM tags WHERE id = ?1 AND user_id = ?2")
        .bind(&[id.into(), user_id.into()])?
        .run()
        .await?;
    Ok(Response::empty()?.with_status(204))
}

/// POST /api/items/:item_id/tags
pub async fn assign_to_item(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let item_id = ctx
        .param("item_id")
        .ok_or_else(|| Error::from("Missing item_id"))?
        .to_string();
    let body: TagAssignment = req.json().await?;
    let d1 = ctx.env.d1("DB")?;

    // Verify item's list belongs to user
    let item_check = d1
        .prepare(
            "SELECT items.id FROM items \
             JOIN lists ON lists.id = items.list_id \
             WHERE items.id = ?1 AND lists.user_id = ?2",
        )
        .bind(&[item_id.clone().into(), user_id.clone().into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if item_check.is_none() {
        return Response::error("Not found", 404);
    }

    // Verify tag belongs to user
    let tag_check = d1
        .prepare("SELECT id FROM tags WHERE id = ?1 AND user_id = ?2")
        .bind(&[body.tag_id.clone().into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if tag_check.is_none() {
        return Response::error("Not found", 404);
    }

    d1.prepare("INSERT OR IGNORE INTO item_tags (item_id, tag_id) VALUES (?1, ?2)")
        .bind(&[item_id.into(), body.tag_id.into()])?
        .run()
        .await?;
    Ok(Response::empty()?.with_status(204))
}

/// DELETE /api/items/:item_id/tags/:tag_id
pub async fn remove_from_item(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let item_id = ctx
        .param("item_id")
        .ok_or_else(|| Error::from("Missing item_id"))?
        .to_string();
    let tag_id = ctx
        .param("tag_id")
        .ok_or_else(|| Error::from("Missing tag_id"))?
        .to_string();
    let d1 = ctx.env.d1("DB")?;

    // Verify item's list belongs to user
    let item_check = d1
        .prepare(
            "SELECT items.id FROM items \
             JOIN lists ON lists.id = items.list_id \
             WHERE items.id = ?1 AND lists.user_id = ?2",
        )
        .bind(&[item_id.clone().into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if item_check.is_none() {
        return Response::error("Not found", 404);
    }

    d1.prepare("DELETE FROM item_tags WHERE item_id = ?1 AND tag_id = ?2")
        .bind(&[item_id.into(), tag_id.into()])?
        .run()
        .await?;
    Ok(Response::empty()?.with_status(204))
}

/// POST /api/lists/:list_id/tags
pub async fn assign_to_list(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = ctx
        .param("list_id")
        .ok_or_else(|| Error::from("Missing list_id"))?
        .to_string();
    let body: TagAssignment = req.json().await?;
    let d1 = ctx.env.d1("DB")?;

    // Verify list belongs to user
    let list_check = d1
        .prepare("SELECT id FROM lists WHERE id = ?1 AND user_id = ?2")
        .bind(&[list_id.clone().into(), user_id.clone().into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if list_check.is_none() {
        return Response::error("Not found", 404);
    }

    // Verify tag belongs to user
    let tag_check = d1
        .prepare("SELECT id FROM tags WHERE id = ?1 AND user_id = ?2")
        .bind(&[body.tag_id.clone().into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if tag_check.is_none() {
        return Response::error("Not found", 404);
    }

    d1.prepare("INSERT OR IGNORE INTO list_tags (list_id, tag_id) VALUES (?1, ?2)")
        .bind(&[list_id.into(), body.tag_id.into()])?
        .run()
        .await?;
    Ok(Response::empty()?.with_status(204))
}

/// DELETE /api/lists/:list_id/tags/:tag_id
pub async fn remove_from_list(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = ctx
        .param("list_id")
        .ok_or_else(|| Error::from("Missing list_id"))?
        .to_string();
    let tag_id = ctx
        .param("tag_id")
        .ok_or_else(|| Error::from("Missing tag_id"))?
        .to_string();
    let d1 = ctx.env.d1("DB")?;

    // Verify list belongs to user
    let list_check = d1
        .prepare("SELECT id FROM lists WHERE id = ?1 AND user_id = ?2")
        .bind(&[list_id.clone().into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if list_check.is_none() {
        return Response::error("Not found", 404);
    }

    d1.prepare("DELETE FROM list_tags WHERE list_id = ?1 AND tag_id = ?2")
        .bind(&[list_id.into(), tag_id.into()])?
        .run()
        .await?;
    Ok(Response::empty()?.with_status(204))
}

/// GET /api/tags/:id/items
pub async fn tag_items(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let tag_id = ctx
        .param("id")
        .ok_or_else(|| Error::from("Missing id"))?
        .to_string();
    let d1 = ctx.env.d1("DB")?;

    // Verify tag belongs to user
    let tag_check = d1
        .prepare("SELECT id FROM tags WHERE id = ?1 AND user_id = ?2")
        .bind(&[tag_id.clone().into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    if tag_check.is_none() {
        return Response::error("Not found", 404);
    }

    // Get all items with this tag, including list name
    let result = d1
        .prepare(
            "SELECT i.id, i.list_id, i.title, i.description, i.completed, i.position, \
             i.quantity, i.actual_quantity, i.unit, i.due_date, i.due_time, \
             i.created_at, i.updated_at, l.name as list_name \
             FROM items i \
             JOIN item_tags it ON it.item_id = i.id \
             JOIN lists l ON l.id = i.list_id \
             WHERE it.tag_id = ?1 \
             ORDER BY l.name, i.position",
        )
        .bind(&[tag_id.into()])?
        .all()
        .await?;

    let rows = result.results::<serde_json::Value>()?;
    Response::from_json(&rows)
}

/// GET /api/tag-links/items
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
