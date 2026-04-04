use kartoteka_shared::{Container, List};
use tracing::instrument;
use worker::*;

use super::CONTAINER_SELECT;

#[instrument(skip_all)]
pub async fn home(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let d1 = ctx.env.d1("DB")?;
    let list_select = crate::handlers::lists::LIST_SELECT;

    let pinned_lists_result = d1
        .prepare(format!(
            "{list_select} WHERE l.user_id = ?1 AND l.pinned = 1 AND l.archived = 0 AND l.parent_list_id IS NULL ORDER BY l.name ASC"
        ))
        .bind(&[user_id.clone().into()])?
        .all()
        .await?;
    let pinned_lists = pinned_lists_result.results::<List>()?;

    let pinned_containers_result = d1
        .prepare(format!(
            "{CONTAINER_SELECT} WHERE c.user_id = ?1 AND c.pinned = 1 ORDER BY c.name ASC"
        ))
        .bind(&[user_id.clone().into()])?
        .all()
        .await?;
    let pinned_containers = pinned_containers_result.results::<Container>()?;

    let recent_lists_result = d1
        .prepare(format!(
            "{list_select} WHERE l.user_id = ?1 AND l.pinned = 0 AND l.last_opened_at IS NOT NULL AND l.archived = 0 AND l.parent_list_id IS NULL ORDER BY l.last_opened_at DESC LIMIT 5"
        ))
        .bind(&[user_id.clone().into()])?
        .all()
        .await?;
    let recent_lists = recent_lists_result.results::<List>()?;

    let recent_containers_result = d1
        .prepare(format!(
            "{CONTAINER_SELECT} WHERE c.user_id = ?1 AND c.pinned = 0 AND c.last_opened_at IS NOT NULL ORDER BY c.last_opened_at DESC LIMIT 5"
        ))
        .bind(&[user_id.clone().into()])?
        .all()
        .await?;
    let recent_containers = recent_containers_result.results::<Container>()?;

    let root_containers_result = d1
        .prepare(format!(
            "{CONTAINER_SELECT} WHERE c.user_id = ?1 AND c.parent_container_id IS NULL ORDER BY c.position ASC, c.created_at ASC"
        ))
        .bind(&[user_id.clone().into()])?
        .all()
        .await?;
    let root_containers = root_containers_result.results::<Container>()?;

    let root_lists_result = d1
        .prepare(format!(
            "{list_select} WHERE l.user_id = ?1 AND l.container_id IS NULL AND l.parent_list_id IS NULL AND l.archived = 0 ORDER BY l.position ASC, l.created_at ASC"
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
