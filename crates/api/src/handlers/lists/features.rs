use crate::error::json_error;
use crate::helpers::*;
use kartoteka_shared::{FeatureConfigRequest, List};
use tracing::instrument;
use worker::*;

use super::LIST_SELECT;

async fn touch_list_updated_at(d1: &D1Database, list_id: &str) -> Result<()> {
    d1.prepare("UPDATE lists SET updated_at = datetime('now') WHERE id = ?1")
        .bind(&[list_id.into()])?
        .run()
        .await?;
    Ok(())
}

#[instrument(skip_all, fields(action = "add_list_feature", list_id = tracing::field::Empty))]
pub async fn add_feature(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "id")?;
    tracing::Span::current().record("list_id", tracing::field::display(&list_id));
    let feature_name = require_param(&ctx, "name")?;

    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "lists", &list_id, &user_id).await? {
        return json_error("list_not_found", 404);
    }

    // Parse config from body (default to {})
    let body: FeatureConfigRequest = req.json().await.unwrap_or(FeatureConfigRequest {
        config: serde_json::json!({}),
    });

    // Validate config is a valid JSON object
    if !body.config.is_object() && !body.config.is_null() {
        return json_error("invalid_config", 400);
    }

    let config_str = body.config.to_string();

    d1.prepare(
        "INSERT OR REPLACE INTO list_features (list_id, feature_name, config) VALUES (?1, ?2, ?3)",
    )
    .bind(&[
        list_id.clone().into(),
        feature_name.into(),
        config_str.into(),
    ])?
    .run()
    .await?;
    touch_list_updated_at(&d1, &list_id).await?;

    // Return updated list
    let list = d1
        .prepare(format!("{LIST_SELECT} WHERE l.id = ?1 AND l.user_id = ?2"))
        .bind(&[list_id.into(), user_id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&list)
}

#[instrument(skip_all, fields(action = "remove_list_feature", list_id = tracing::field::Empty))]
pub async fn remove_feature(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let list_id = require_param(&ctx, "id")?;
    tracing::Span::current().record("list_id", tracing::field::display(&list_id));
    let feature_name = require_param(&ctx, "name")?;

    let d1 = ctx.env.d1("DB")?;

    if !check_ownership(&d1, "lists", &list_id, &user_id).await? {
        return json_error("list_not_found", 404);
    }

    d1.prepare("DELETE FROM list_features WHERE list_id = ?1 AND feature_name = ?2")
        .bind(&[list_id.clone().into(), feature_name.into()])?
        .run()
        .await?;
    touch_list_updated_at(&d1, &list_id).await?;

    // Return updated list
    let list = d1
        .prepare(format!("{LIST_SELECT} WHERE l.id = ?1 AND l.user_id = ?2"))
        .bind(&[list_id.into(), user_id.into()])?
        .first::<List>(None)
        .await?
        .ok_or_else(|| Error::from("Not found"))?;

    Response::from_json(&list)
}
