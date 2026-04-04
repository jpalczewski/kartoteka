use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::Container;
use tracing::instrument;
use worker::*;

use super::CONTAINER_SELECT;

#[instrument(skip_all, fields(action = "toggle_container_pin", container_id = tracing::field::Empty))]
pub async fn toggle_pin(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    tracing::Span::current().record("container_id", tracing::field::display(&id));
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
