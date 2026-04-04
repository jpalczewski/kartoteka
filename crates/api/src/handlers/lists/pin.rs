use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::List;
use tracing::instrument;
use worker::*;

use super::LIST_SELECT;

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
