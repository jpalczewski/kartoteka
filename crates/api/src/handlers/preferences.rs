use crate::error::json_error;
use serde::{Deserialize, Serialize};
use worker::*;

#[derive(Serialize)]
struct PreferencesResponse {
    locale: String,
}

#[derive(Deserialize)]
struct UpdatePreferencesBody {
    locale: String,
}

pub async fn get_preferences(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();

    let db = ctx.env.d1("DB")?;
    let result = db
        .prepare("SELECT locale FROM user_preferences WHERE user_id = ?1")
        .bind(&[user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    let locale = result
        .and_then(|row| {
            row.get("locale")
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .unwrap_or_else(|| "en".to_string());

    Response::from_json(&PreferencesResponse { locale })
}

pub async fn put_preferences(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();

    let body: UpdatePreferencesBody = req.json().await?;

    if !["en", "pl"].contains(&body.locale.as_str()) {
        return json_error("invalid_locale", 400);
    }

    let db = ctx.env.d1("DB")?;
    db.prepare(
        "INSERT INTO user_preferences (user_id, locale, updated_at) \
         VALUES (?1, ?2, datetime('now')) \
         ON CONFLICT(user_id) DO UPDATE SET locale = ?2, updated_at = datetime('now')",
    )
    .bind(&[user_id.into(), body.locale.into()])?
    .run()
    .await?;

    Response::from_json(&serde_json::json!({ "ok": true }))
}
