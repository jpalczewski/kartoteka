use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::{Container, List};
use tracing::instrument;
use worker::*;

use super::CONTAINER_SELECT;

#[instrument(skip_all)]
pub async fn get_children(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "containers", &id, &user_id).await? {
        return json_error("container_not_found", 404);
    }

    let sub_result = d1
        .prepare(format!(
            "{CONTAINER_SELECT} WHERE c.parent_container_id = ?1 ORDER BY c.position ASC"
        ))
        .bind(&[id.clone().into()])?
        .all()
        .await?;
    let sub_containers = sub_result.results::<Container>()?;

    let list_select = crate::handlers::lists::LIST_SELECT;
    let list_result = d1
        .prepare(format!(
            "{list_select} WHERE l.container_id = ?1 AND l.parent_list_id IS NULL AND l.archived = 0 ORDER BY l.position ASC, l.created_at ASC"
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
