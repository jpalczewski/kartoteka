use crate::helpers::require_param;
use kartoteka_shared::*;
use std::collections::HashMap;
use tracing::instrument;
use worker::*;

#[instrument(skip_all)]
pub async fn list_all(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let d1 = ctx.env.d1("DB")?;

    let rows = d1
        .prepare("SELECT key, value FROM user_settings WHERE user_id = ?1")
        .bind(&[user_id.into()])?
        .all()
        .await?
        .results::<serde_json::Value>()?;

    let mut map: HashMap<String, serde_json::Value> = HashMap::new();
    for row in rows {
        if let (Some(key), Some(raw_value)) = (
            row.get("key").and_then(|v| v.as_str()).map(String::from),
            row.get("value").and_then(|v| v.as_str()),
        ) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw_value) {
                map.insert(key, parsed);
            }
        }
    }
    Response::from_json(&map)
}

#[instrument(skip_all, fields(action = "upsert_setting"))]
pub async fn upsert(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let key = require_param(&ctx, "key")?;
    let body: UpsertSettingRequest = req.json().await?;
    let d1 = ctx.env.d1("DB")?;

    let value_str = body.value.to_string();
    d1.prepare(
        "INSERT OR REPLACE INTO user_settings (user_id, key, value, updated_at) \
         VALUES (?1, ?2, ?3, datetime('now'))",
    )
    .bind(&[user_id.into(), key.into(), value_str.into()])?
    .run()
    .await?;

    Ok(Response::empty()?.with_status(204))
}

#[instrument(skip_all, fields(action = "delete_setting"))]
pub async fn delete(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let key = require_param(&ctx, "key")?;
    let d1 = ctx.env.d1("DB")?;

    d1.prepare("DELETE FROM user_settings WHERE user_id = ?1 AND key = ?2")
        .bind(&[user_id.into(), key.into()])?
        .run()
        .await?;

    Ok(Response::empty()?.with_status(204))
}
