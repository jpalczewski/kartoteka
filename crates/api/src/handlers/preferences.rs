use crate::error::json_error;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use worker::*;

#[derive(Serialize)]
struct PreferencesResponse {
    locale: String,
}

#[derive(Deserialize)]
struct UpdatePreferencesBody {
    locale: String,
}

#[instrument(skip_all)]
pub async fn get_preferences(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();

    let db = ctx.env.d1("DB")?;
    let result = db
        .prepare("SELECT value FROM user_settings WHERE user_id = ?1 AND key = 'locale'")
        .bind(&[user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    let locale = result
        .and_then(|row| {
            let raw = row.get("value")?.as_str().map(String::from)?;
            serde_json::from_str::<String>(&raw).ok()
        })
        .unwrap_or_else(|| "en".to_string());

    Response::from_json(&PreferencesResponse { locale })
}

#[instrument(skip_all, fields(action = "update_preferences"))]
pub async fn put_preferences(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();

    let body: UpdatePreferencesBody = req.json().await?;

    if !["en", "pl"].contains(&body.locale.as_str()) {
        return json_error("invalid_locale", 400);
    }

    let value_json = serde_json::to_string(&body.locale).unwrap_or_default();

    let db = ctx.env.d1("DB")?;
    db.prepare(
        "INSERT OR REPLACE INTO user_settings (user_id, key, value, updated_at) \
         VALUES (?1, 'locale', ?2, datetime('now'))",
    )
    .bind(&[user_id.into(), value_json.into()])?
    .run()
    .await?;

    Response::from_json(&serde_json::json!({ "ok": true }))
}
