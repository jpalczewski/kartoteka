use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::*;
use tracing::instrument;
use worker::*;

use super::{LIST_SELECT, ensure_parent_list_target, list_has_sublists, placement_filter};

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
        Ok(lists) => Response::from_json(&serde_json::json!({ "moved_lists": lists })),
        Err(err) if err.to_string() == "list_not_found" => json_error("list_not_found", 404),
        Err(err) if err.to_string() == "container_not_found" => {
            json_error("container_not_found", 404)
        }
        Err(err) => json_error(err.to_string().as_str(), 400),
    }
}
