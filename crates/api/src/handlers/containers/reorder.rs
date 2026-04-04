use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::{Container, MoveContainerRequest, ReorderContainersRequest};
use tracing::instrument;
use worker::*;

use super::CONTAINER_SELECT;

#[instrument(skip_all, fields(action = "reorder_containers", container_count = tracing::field::Empty))]
pub async fn reorder(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let body: ReorderContainersRequest = req.json().await?;
    tracing::Span::current().record("container_count", body.container_ids.len());

    if let Err(code) = body.validate() {
        return json_error(code, 400);
    }

    let d1 = ctx.env.d1("DB")?;

    if let Some(ref parent_id) = body.parent_container_id {
        if let Some(err) = validate_parent_container(&d1, parent_id, &user_id).await? {
            return Ok(err);
        }
    }

    let current_ids = match body.parent_container_id.as_deref() {
        Some(parent_id) => {
            fetch_ordered_ids(
                &d1,
                "SELECT id FROM containers \
                 WHERE user_id = ?1 AND parent_container_id = ?2 \
                 ORDER BY position ASC, created_at ASC",
                &[user_id.clone().into(), parent_id.into()],
            )
            .await?
        }
        None => {
            fetch_ordered_ids(
                &d1,
                "SELECT id FROM containers \
                 WHERE user_id = ?1 AND parent_container_id IS NULL \
                 ORDER BY position ASC, created_at ASC",
                &[user_id.clone().into()],
            )
            .await?
        }
    };
    if !ids_match_exact_set(&current_ids, &body.container_ids) {
        return json_error("invalid_container_reorder", 400);
    }

    apply_positions(&d1, "containers", &body.container_ids).await?;
    Ok(Response::empty()?.with_status(204))
}

#[instrument(skip_all, fields(action = "move_container", container_id = tracing::field::Empty))]
pub async fn move_container(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("container_id", tracing::field::display(&id));
    let body: MoveContainerRequest = req.json().await?;
    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "containers", &id, &user_id).await? {
        return json_error("container_not_found", 404);
    }

    if let Some(ref parent_id) = body.parent_container_id {
        if parent_id == &id {
            return json_error("invalid_container_move", 400);
        }
        if let Some(err) = validate_parent_container(&d1, parent_id, &user_id).await? {
            return Ok(err);
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
