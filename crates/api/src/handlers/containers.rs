use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::{
    Container, ContainerDetail, CreateContainerRequest, List, MoveContainerRequest,
    UpdateContainerRequest,
};
use tracing::instrument;
use wasm_bindgen::JsValue;
use worker::*;

const CONTAINER_SELECT: &str = "\
    SELECT c.id, c.user_id, c.name, c.description, c.status, \
    c.parent_container_id, c.position, c.pinned, c.last_opened_at, \
    c.created_at, c.updated_at \
    FROM containers c";

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
    tracing::Span::current().record("container_id", &tracing::field::display(&id));
    let d1 = ctx.env.d1("DB")?;

    // Validate: parent must not be a project (status != NULL)
    if let Some(ref parent_id) = body.parent_container_id {
        let parent = d1
            .prepare("SELECT status FROM containers WHERE id = ?1 AND user_id = ?2")
            .bind(&[parent_id.clone().into(), user_id.clone().into()])?
            .first::<serde_json::Value>(None)
            .await?;
        match parent {
            None => return json_error("container_not_found", 404),
            Some(ref p) => {
                if !p.get("status").map(|v| v.is_null()).unwrap_or(true) {
                    return json_error("invalid_container_hierarchy", 400);
                }
            }
        }
    }

    let status_val: JsValue = match &body.status {
        Some(s) => {
            let s_str = serde_json::to_value(s)
                .map_err(|e| Error::from(e.to_string()))?
                .as_str()
                .unwrap_or("active")
                .to_string();
            JsValue::from(s_str.as_str())
        }
        None => JsValue::NULL,
    };

    let parent_val = opt_str_to_js(&body.parent_container_id);

    // Position: max + 1 among siblings
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

    // Track last opened
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

    // Compute progress (item-level + list-level)
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
    tracing::Span::current().record("container_id", &tracing::field::display(&id));
    let body: UpdateContainerRequest = req.json().await?;
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

    if let Some(description) = &body.description {
        d1.prepare(
            "UPDATE containers SET description = ?1, updated_at = datetime('now') WHERE id = ?2",
        )
        .bind(&[description.clone().into(), id.clone().into()])?
        .run()
        .await?;
    }

    if let Some(status_opt) = &body.status {
        let status_val: JsValue = match status_opt {
            Some(s) => {
                let s_str = serde_json::to_value(s)
                    .map_err(|e| Error::from(e.to_string()))?
                    .as_str()
                    .unwrap_or("active")
                    .to_string();
                JsValue::from(s_str.as_str())
            }
            None => JsValue::NULL,
        };
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
    tracing::Span::current().record("container_id", &tracing::field::display(&id));
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

#[instrument(skip_all)]
pub async fn get_children(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "containers", &id, &user_id).await? {
        return json_error("container_not_found", 404);
    }

    // Get sub-containers
    let sub_result = d1
        .prepare(format!(
            "{CONTAINER_SELECT} WHERE c.parent_container_id = ?1 ORDER BY c.position ASC"
        ))
        .bind(&[id.clone().into()])?
        .all()
        .await?;
    let sub_containers = sub_result.results::<Container>()?;

    // Get lists in this container
    let list_select = super::lists::LIST_SELECT;
    let list_result = d1
        .prepare(format!(
            "{list_select} WHERE l.container_id = ?1 AND l.parent_list_id IS NULL AND l.archived = 0 ORDER BY l.updated_at DESC"
        ))
        .bind(&[id.into()])?
        .all()
        .await?;
    let lists = list_result.results::<List>()?;

    let resp = serde_json::json!({
        "containers": sub_containers,
        "lists": lists,
    });

    Response::from_json(&resp)
}

#[instrument(skip_all, fields(action = "move_container", container_id = tracing::field::Empty))]
pub async fn move_container(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("container_id", &tracing::field::display(&id));
    let body: MoveContainerRequest = req.json().await?;
    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "containers", &id, &user_id).await? {
        return json_error("container_not_found", 404);
    }

    // Validate new parent is not a project
    if let Some(ref parent_id) = body.parent_container_id {
        if parent_id == &id {
            return json_error("invalid_container_move", 400);
        }
        let parent = d1
            .prepare("SELECT status FROM containers WHERE id = ?1 AND user_id = ?2")
            .bind(&[parent_id.clone().into(), user_id.clone().into()])?
            .first::<serde_json::Value>(None)
            .await?;
        match parent {
            None => return json_error("container_not_found", 404),
            Some(ref p) => {
                if !p.get("status").map(|v| v.is_null()).unwrap_or(true) {
                    return json_error("invalid_container_hierarchy", 400);
                }
            }
        }
    }

    let parent_val = opt_str_to_js(&body.parent_container_id);

    d1.prepare(
        "UPDATE containers SET parent_container_id = ?1, updated_at = datetime('now') WHERE id = ?2",
    )
    .bind(&[parent_val, id.clone().into()])?
    .run()
    .await?;

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

#[instrument(skip_all, fields(action = "toggle_container_pin", container_id = tracing::field::Empty))]
pub async fn toggle_pin(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("container_id", &tracing::field::display(&id));
    let d1 = ctx.env.d1("DB")?;

    if toggle_bool_field(&d1, "containers", "pinned", &id, &user_id)
        .await?
        .is_none()
    {
        return json_error("container_not_found", 404);
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

// === Home endpoint ===

#[instrument(skip_all)]
pub async fn home(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let d1 = ctx.env.d1("DB")?;
    let list_select = super::lists::LIST_SELECT;

    // Pinned lists
    let pinned_lists_result = d1
        .prepare(format!(
            "{list_select} WHERE l.user_id = ?1 AND l.pinned = 1 AND l.archived = 0 AND l.parent_list_id IS NULL ORDER BY l.name ASC"
        ))
        .bind(&[user_id.clone().into()])?
        .all()
        .await?;
    let pinned_lists = pinned_lists_result.results::<List>()?;

    // Pinned containers
    let pinned_containers_result = d1
        .prepare(format!(
            "{CONTAINER_SELECT} WHERE c.user_id = ?1 AND c.pinned = 1 ORDER BY c.name ASC"
        ))
        .bind(&[user_id.clone().into()])?
        .all()
        .await?;
    let pinned_containers = pinned_containers_result.results::<Container>()?;

    // Recent lists (not pinned, has last_opened_at, top 5)
    let recent_lists_result = d1
        .prepare(format!(
            "{list_select} WHERE l.user_id = ?1 AND l.pinned = 0 AND l.last_opened_at IS NOT NULL AND l.archived = 0 AND l.parent_list_id IS NULL ORDER BY l.last_opened_at DESC LIMIT 5"
        ))
        .bind(&[user_id.clone().into()])?
        .all()
        .await?;
    let recent_lists = recent_lists_result.results::<List>()?;

    // Recent containers (not pinned, has last_opened_at, top 5)
    let recent_containers_result = d1
        .prepare(format!(
            "{CONTAINER_SELECT} WHERE c.user_id = ?1 AND c.pinned = 0 AND c.last_opened_at IS NOT NULL ORDER BY c.last_opened_at DESC LIMIT 5"
        ))
        .bind(&[user_id.clone().into()])?
        .all()
        .await?;
    let recent_containers = recent_containers_result.results::<Container>()?;

    // Root containers (no parent)
    let root_containers_result = d1
        .prepare(format!(
            "{CONTAINER_SELECT} WHERE c.user_id = ?1 AND c.parent_container_id IS NULL ORDER BY c.position ASC, c.created_at ASC"
        ))
        .bind(&[user_id.clone().into()])?
        .all()
        .await?;
    let root_containers = root_containers_result.results::<Container>()?;

    // Root lists (no container, no parent list, not archived)
    let root_lists_result = d1
        .prepare(format!(
            "{list_select} WHERE l.user_id = ?1 AND l.container_id IS NULL AND l.parent_list_id IS NULL AND l.archived = 0 ORDER BY l.updated_at DESC"
        ))
        .bind(&[user_id.into()])?
        .all()
        .await?;
    let root_lists = root_lists_result.results::<List>()?;

    let resp = serde_json::json!({
        "pinned_lists": pinned_lists,
        "pinned_containers": pinned_containers,
        "recent_lists": recent_lists,
        "recent_containers": recent_containers,
        "root_containers": root_containers,
        "root_lists": root_lists,
    });

    Response::from_json(&resp)
}
