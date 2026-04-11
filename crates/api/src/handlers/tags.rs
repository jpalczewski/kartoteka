use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::*;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use wasm_bindgen::JsValue;
use worker::*;

const TAG_ITEM_COLS: &str = "i.id, i.list_id, i.title, i.description, i.completed, i.position, \
    i.quantity, i.actual_quantity, i.unit, i.start_date, i.start_time, i.deadline, i.deadline_time, i.hard_deadline, \
    i.created_at, i.updated_at, l.name as list_name, l.list_type";

const DEFAULT_LIMIT: u32 = 100;
const MAX_LIMIT: u32 = 100;

#[derive(Debug, Deserialize)]
struct TagEntityListRow {
    id: String,
    name: String,
    description: Option<String>,
    updated_at: String,
    list_type: ListType,
    archived: serde_json::Value,
    container_id: Option<String>,
    parent_list_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TagEntityKind {
    Item,
    List,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TagsCursorParams {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TagsCursorLast {
    pub name_sort: String,
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
    cursor: Option<&TagsCursorLast>,
) -> Result<CursorPage<Tag>> {
    let mut sql =
        "SELECT id, user_id, name, color, parent_tag_id, created_at FROM tags WHERE user_id = ?1"
            .to_string();
    let mut params: Vec<JsValue> = vec![user_id.into()];

    if let Some(cursor) = cursor {
        sql.push_str(&format!(
            " AND ((LOWER(name) > ?{}) OR (LOWER(name) = ?{} AND id > ?{}))",
            params.len() + 1,
            params.len() + 2,
            params.len() + 3,
        ));
        params.push(cursor.name_sort.clone().into());
        params.push(cursor.name_sort.clone().into());
        params.push(cursor.id.clone().into());
    }

    params.push(((limit + 1) as i32).into());
    sql.push_str(&format!(
        " ORDER BY LOWER(name) ASC, id ASC LIMIT ?{}",
        params.len()
    ));

    let mut items = d1
        .prepare(&sql)
        .bind(&params)?
        .all()
        .await?
        .results::<Tag>()?;
    let next_cursor = if items.len() > limit as usize {
        items.truncate(limit as usize);
        items
            .last()
            .map(|last| {
                crate::cursor::encode_cursor(
                    "tags",
                    limit,
                    &TagsCursorParams::default(),
                    &TagsCursorLast {
                        name_sort: last.name.to_lowercase(),
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

async fn apply_tag_links(
    d1: &D1Database,
    user_id: &str,
    body: SetTagLinksRequest,
) -> Result<serde_json::Value> {
    if let Err(code) = body.validate() {
        return Err(Error::from(code));
    }

    let SetTagLinksRequest {
        action,
        tag_ids,
        item_ids,
        list_ids,
    } = body;

    let tag_ids = dedupe_ids(&tag_ids);
    let (target_kind, target_ids) = if let Some(item_ids) = item_ids {
        ("item", dedupe_ids(&item_ids))
    } else if let Some(list_ids) = list_ids {
        ("list", dedupe_ids(&list_ids))
    } else {
        return Err(Error::from("provide item_ids or list_ids"));
    };

    for tag_id in &tag_ids {
        if !check_ownership(d1, "tags", tag_id, user_id).await? {
            return Err(Error::from("tag_not_found"));
        }
    }

    match target_kind {
        "item" => {
            for item_id in &target_ids {
                if !check_item_ownership(d1, item_id, user_id).await? {
                    return Err(Error::from("item_not_found"));
                }
            }
        }
        "list" => {
            for list_id in &target_ids {
                if !check_ownership(d1, "lists", list_id, user_id).await? {
                    return Err(Error::from("list_not_found"));
                }
            }
        }
        _ => unreachable!("validated above"),
    }

    let requested_links = tag_ids.len() * target_ids.len();
    let mut applied_links = 0usize;
    let mut skipped_duplicates = 0usize;

    for target_id in &target_ids {
        for tag_id in &tag_ids {
            let exists = match target_kind {
                "item" => d1
                    .prepare("SELECT 1 FROM item_tags WHERE item_id = ?1 AND tag_id = ?2 LIMIT 1")
                    .bind(&[target_id.clone().into(), tag_id.clone().into()])?
                    .first::<serde_json::Value>(None)
                    .await?
                    .is_some(),
                "list" => d1
                    .prepare("SELECT 1 FROM list_tags WHERE list_id = ?1 AND tag_id = ?2 LIMIT 1")
                    .bind(&[target_id.clone().into(), tag_id.clone().into()])?
                    .first::<serde_json::Value>(None)
                    .await?
                    .is_some(),
                _ => unreachable!("validated above"),
            };

            match action {
                TagLinkAction::Assign if exists => skipped_duplicates += 1,
                TagLinkAction::Remove if !exists => skipped_duplicates += 1,
                TagLinkAction::Assign => {
                    match target_kind {
                        "item" => {
                            d1.prepare("INSERT INTO item_tags (item_id, tag_id) VALUES (?1, ?2)")
                                .bind(&[target_id.clone().into(), tag_id.clone().into()])?
                                .run()
                                .await?;
                        }
                        "list" => {
                            d1.prepare("INSERT INTO list_tags (list_id, tag_id) VALUES (?1, ?2)")
                                .bind(&[target_id.clone().into(), tag_id.clone().into()])?
                                .run()
                                .await?;
                        }
                        _ => unreachable!("validated above"),
                    }
                    applied_links += 1;
                }
                TagLinkAction::Remove => {
                    match target_kind {
                        "item" => {
                            d1.prepare("DELETE FROM item_tags WHERE item_id = ?1 AND tag_id = ?2")
                                .bind(&[target_id.clone().into(), tag_id.clone().into()])?
                                .run()
                                .await?;
                        }
                        "list" => {
                            d1.prepare("DELETE FROM list_tags WHERE list_id = ?1 AND tag_id = ?2")
                                .bind(&[target_id.clone().into(), tag_id.clone().into()])?
                                .run()
                                .await?;
                        }
                        _ => unreachable!("validated above"),
                    }
                    applied_links += 1;
                }
            }
        }
    }

    Ok(serde_json::json!({
        "action": action,
        "target_kind": target_kind,
        "tag_ids": tag_ids,
        "target_ids": target_ids,
        "requested_links": requested_links,
        "applied_links": applied_links,
        "skipped_duplicates": skipped_duplicates,
    }))
}

/// GET /api/tags
#[instrument(skip_all)]
pub async fn list_all(req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.as_str();
    let limit = parse_limit_query(query_param(&req.url()?, "limit"))?;
    let d1 = ctx.env.d1("DB")?;
    let page = list_all_page(&d1, user_id, limit, None).await?;
    Response::from_json(&page)
}

/// POST /api/tags
#[instrument(skip_all, fields(action = "create_tag", tag_id = tracing::field::Empty))]
pub async fn create(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let body: CreateTagRequest = match parse_json_body(&mut req).await {
        Ok(body) => body,
        Err(resp) => return Ok(resp),
    };
    let id = uuid::Uuid::new_v4().to_string();
    tracing::Span::current().record("tag_id", tracing::field::display(&id));

    let parent_val = opt_str_to_js(&body.parent_tag_id);
    let color = body.color.unwrap_or_else(random_hex_color);

    let d1 = ctx.env.d1("DB")?;
    d1.prepare(
        "INSERT INTO tags (id, user_id, name, color, parent_tag_id) VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(&[
        id.clone().into(),
        user_id.clone().into(),
        body.name.into(),
        color.into(),
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
    tracing::Span::current().record("tag_id", tracing::field::display(&id));
    let body: UpdateTagRequest = match parse_json_body(&mut req).await {
        Ok(body) => body,
        Err(resp) => return Ok(resp),
    };
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
    tracing::Span::current().record("tag_id", tracing::field::display(&id));
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
    tracing::Span::current().record("tag_id", tracing::field::display(&source_id));
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
    let batch = SetTagLinksRequest {
        action: TagLinkAction::Assign,
        tag_ids: vec![body.tag_id],
        item_ids: Some(vec![item_id]),
        list_ids: None,
    };
    match apply_tag_links(&d1, &user_id, batch).await {
        Ok(_) => Ok(Response::empty()?.with_status(204)),
        Err(err) if err.to_string() == "item_not_found" => json_error("item_not_found", 404),
        Err(err) if err.to_string() == "tag_not_found" => json_error("tag_not_found", 404),
        Err(err) => json_error(err.to_string().as_str(), 400),
    }
}

/// DELETE /api/items/:item_id/tags/:tag_id
#[instrument(skip_all, fields(action = "remove_tag_from_item"))]
pub async fn remove_from_item(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let item_id = require_param(&ctx, "item_id")?;
    let tag_id = require_param(&ctx, "tag_id")?;
    let d1 = ctx.env.d1("DB")?;
    let batch = SetTagLinksRequest {
        action: TagLinkAction::Remove,
        tag_ids: vec![tag_id],
        item_ids: Some(vec![item_id]),
        list_ids: None,
    };
    match apply_tag_links(&d1, &user_id, batch).await {
        Ok(_) => Ok(Response::empty()?.with_status(204)),
        Err(err) if err.to_string() == "item_not_found" => json_error("item_not_found", 404),
        Err(err) if err.to_string() == "tag_not_found" => json_error("tag_not_found", 404),
        Err(err) => json_error(err.to_string().as_str(), 400),
    }
}

/// POST /api/lists/:list_id/tags
#[instrument(skip_all, fields(action = "assign_tag_to_list"))]
pub async fn assign_to_list(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "list_id")?;
    let body: TagAssignment = req.json().await?;
    let d1 = ctx.env.d1("DB")?;
    let batch = SetTagLinksRequest {
        action: TagLinkAction::Assign,
        tag_ids: vec![body.tag_id],
        item_ids: None,
        list_ids: Some(vec![list_id]),
    };
    match apply_tag_links(&d1, &user_id, batch).await {
        Ok(_) => Ok(Response::empty()?.with_status(204)),
        Err(err) if err.to_string() == "list_not_found" => json_error("list_not_found", 404),
        Err(err) if err.to_string() == "tag_not_found" => json_error("tag_not_found", 404),
        Err(err) => json_error(err.to_string().as_str(), 400),
    }
}

/// DELETE /api/lists/:list_id/tags/:tag_id
#[instrument(skip_all, fields(action = "remove_tag_from_list"))]
pub async fn remove_from_list(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "list_id")?;
    let tag_id = require_param(&ctx, "tag_id")?;
    let d1 = ctx.env.d1("DB")?;
    let batch = SetTagLinksRequest {
        action: TagLinkAction::Remove,
        tag_ids: vec![tag_id],
        item_ids: None,
        list_ids: Some(vec![list_id]),
    };
    match apply_tag_links(&d1, &user_id, batch).await {
        Ok(_) => Ok(Response::empty()?.with_status(204)),
        Err(err) if err.to_string() == "list_not_found" => json_error("list_not_found", 404),
        Err(err) if err.to_string() == "tag_not_found" => json_error("tag_not_found", 404),
        Err(err) => json_error(err.to_string().as_str(), 400),
    }
}

#[instrument(
    skip_all,
    fields(
        action = "set_tag_links",
        tag_count = tracing::field::Empty,
        target_count = tracing::field::Empty,
        target_kind = tracing::field::Empty
    )
)]
pub async fn set_links(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let body: SetTagLinksRequest = match parse_json_body(&mut req).await {
        Ok(body) => body,
        Err(resp) => return Ok(resp),
    };
    tracing::Span::current().record("tag_count", body.tag_ids.len());
    tracing::Span::current().record(
        "target_count",
        body.item_ids.as_ref().map_or_else(
            || body.list_ids.as_ref().map_or(0, std::vec::Vec::len),
            std::vec::Vec::len,
        ),
    );
    tracing::Span::current().record(
        "target_kind",
        tracing::field::display(if body.item_ids.is_some() {
            "item"
        } else if body.list_ids.is_some() {
            "list"
        } else {
            "unknown"
        }),
    );
    let d1 = ctx.env.d1("DB")?;

    match apply_tag_links(&d1, &user_id, body).await {
        Ok(summary) => Response::from_json(&summary),
        Err(err) if err.to_string() == "item_not_found" => json_error("item_not_found", 404),
        Err(err) if err.to_string() == "list_not_found" => json_error("list_not_found", 404),
        Err(err) if err.to_string() == "tag_not_found" => json_error("tag_not_found", 404),
        Err(err) => json_error(err.to_string().as_str(), 400),
    }
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
        let sql = format!(
            "WITH RECURSIVE tag_tree AS ( \
             SELECT id FROM tags WHERE id = ?1 AND user_id = ?2 \
             UNION ALL \
             SELECT t.id FROM tags t JOIN tag_tree tt ON t.parent_tag_id = tt.id WHERE t.user_id = ?2 \
             ) \
             SELECT DISTINCT {cols}, NULL as date_type \
             FROM items i \
             JOIN item_tags it ON it.item_id = i.id \
             JOIN tag_tree tt ON it.tag_id = tt.id \
             JOIN lists l ON l.id = i.list_id \
             ORDER BY l.name, i.position",
            cols = TAG_ITEM_COLS,
        );
        d1.prepare(&sql)
            .bind(&[tag_id.into(), user_id.into()])?
            .all()
            .await?
            .results::<DateItem>()?
    } else {
        let sql = format!(
            "SELECT {cols}, NULL as date_type \
             FROM items i \
             JOIN item_tags it ON it.item_id = i.id \
             JOIN lists l ON l.id = i.list_id \
             WHERE it.tag_id = ?1 AND l.user_id = ?2 \
             ORDER BY l.name, i.position",
            cols = TAG_ITEM_COLS,
        );
        d1.prepare(&sql)
            .bind(&[tag_id.into(), user_id.into()])?
            .all()
            .await?
            .results::<DateItem>()?
    };

    Response::from_json(&rows)
}

fn parse_recursive_query(url: &Url) -> bool {
    url.query_pairs()
        .find(|(k, _)| k == "recursive")
        .map(|(_, v)| v != "false")
        .unwrap_or(true)
}

fn parse_entity_type_query(url: &Url) -> std::result::Result<Option<TagEntityKind>, &'static str> {
    match url
        .query_pairs()
        .find(|(k, _)| k == "entity_type")
        .map(|(_, value)| value.to_string())
    {
        None => Ok(None),
        Some(value) if value == "item" => Ok(Some(TagEntityKind::Item)),
        Some(value) if value == "list" => Ok(Some(TagEntityKind::List)),
        Some(_) => Err("invalid_entity_type"),
    }
}

fn tag_tree_cte_sql(recursive: bool) -> &'static str {
    if recursive {
        "WITH RECURSIVE tag_tree AS ( \
         SELECT id FROM tags WHERE id = ?1 AND user_id = ?2 \
         UNION ALL \
         SELECT t.id FROM tags t JOIN tag_tree tt ON t.parent_tag_id = tt.id WHERE t.user_id = ?2 \
         )"
    } else {
        "WITH tag_tree AS (SELECT id FROM tags WHERE id = ?1 AND user_id = ?2)"
    }
}

async fn fetch_tag_item_entities(
    d1: &D1Database,
    user_id: &str,
    tag_id: &str,
    recursive: bool,
) -> Result<Vec<SearchEntityResult>> {
    let sql = format!(
        "{tag_tree} \
         SELECT DISTINCT {cols}, NULL as date_type \
         FROM items i \
         JOIN item_tags it ON it.item_id = i.id \
         JOIN tag_tree tt ON it.tag_id = tt.id \
         JOIN lists l ON l.id = i.list_id \
         WHERE l.user_id = ?2 \
         ORDER BY i.updated_at DESC, l.name ASC, i.id ASC",
        tag_tree = tag_tree_cte_sql(recursive),
        cols = TAG_ITEM_COLS,
    );

    let rows = d1
        .prepare(&sql)
        .bind(&[tag_id.into(), user_id.into()])?
        .all()
        .await?
        .results::<DateItem>()?;

    Ok(rows
        .into_iter()
        .map(|row| SearchEntityResult {
            entity_type: SearchEntityType::Item,
            id: row.id,
            name: row.title,
            description: row.description,
            updated_at: row.updated_at,
            list_id: Some(row.list_id),
            list_name: Some(row.list_name),
            list_type: Some(row.list_type),
            archived: None,
            completed: Some(row.completed),
            container_id: None,
            parent_list_id: None,
            parent_container_id: None,
            status: None,
        })
        .collect())
}

async fn fetch_tag_list_entities(
    d1: &D1Database,
    user_id: &str,
    tag_id: &str,
    recursive: bool,
) -> Result<Vec<SearchEntityResult>> {
    let sql = format!(
        "{tag_tree} \
         SELECT DISTINCT \
            l.id, l.name, l.description, l.updated_at, l.list_type, l.archived, l.container_id, l.parent_list_id \
         FROM lists l \
         JOIN list_tags lt ON lt.list_id = l.id \
         JOIN tag_tree tt ON lt.tag_id = tt.id \
         WHERE l.user_id = ?2 \
         ORDER BY l.updated_at DESC, l.name ASC, l.id ASC",
        tag_tree = tag_tree_cte_sql(recursive),
    );

    let rows = d1
        .prepare(&sql)
        .bind(&[tag_id.into(), user_id.into()])?
        .all()
        .await?
        .results::<TagEntityListRow>()?;

    Ok(rows
        .into_iter()
        .map(|row| SearchEntityResult {
            entity_type: SearchEntityType::List,
            id: row.id,
            name: row.name,
            description: row.description,
            updated_at: row.updated_at,
            list_id: None,
            list_name: None,
            list_type: Some(row.list_type),
            archived: match row.archived {
                serde_json::Value::Bool(value) => Some(value),
                serde_json::Value::Number(value) => Some(value.as_f64().unwrap_or(0.0) != 0.0),
                _ => None,
            },
            completed: None,
            container_id: row.container_id,
            parent_list_id: row.parent_list_id,
            parent_container_id: None,
            status: None,
        })
        .collect())
}

/// GET /api/tags/:id/entities
#[instrument(skip_all, fields(action = "get_tag_entities", tag_id = tracing::field::Empty))]
pub async fn tag_entities(req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let tag_id = require_param(&ctx, "id")?;
    tracing::Span::current().record("tag_id", tracing::field::display(&tag_id));
    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "tags", &tag_id, &user_id).await? {
        return json_error("tag_not_found", 404);
    }

    let url = req.url()?;
    let recursive = parse_recursive_query(&url);
    let entity_type = match parse_entity_type_query(&url) {
        Ok(entity_type) => entity_type,
        Err(code) => return json_error(code, 400),
    };

    let mut entities = Vec::new();
    if entity_type != Some(TagEntityKind::List) {
        entities.extend(fetch_tag_item_entities(&d1, &user_id, &tag_id, recursive).await?);
    }
    if entity_type != Some(TagEntityKind::Item) {
        entities.extend(fetch_tag_list_entities(&d1, &user_id, &tag_id, recursive).await?);
    }

    entities.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| {
                format!("{:?}", left.entity_type).cmp(&format!("{:?}", right.entity_type))
            })
            .then_with(|| left.id.cmp(&right.id))
    });

    Response::from_json(&entities)
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
