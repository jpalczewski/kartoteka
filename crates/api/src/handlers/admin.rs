use crate::helpers::{opt_str_to_js, require_admin, require_param};
use kartoteka_shared::dto::requests::{CreateInvitationCodeRequest, UpsertSettingRequest};
use kartoteka_shared::models::{InvitationCode, UserSetting};
use tracing::instrument;
use uuid::Uuid;
use worker::*;

// ── Instance Settings ──────────────────────────────────────────────────────

#[instrument(skip_all)]
pub async fn list_settings(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let d1 = ctx.env.d1("DB")?;
    if let Some(r) = require_admin(&d1, &user_id).await? {
        return Ok(r);
    }

    let rows = d1
        .prepare("SELECT key, value FROM instance_settings ORDER BY key")
        .all()
        .await?
        .results::<serde_json::Value>()?;

    let settings: Vec<UserSetting> = rows
        .into_iter()
        .filter_map(|row| {
            let key = row.get("key")?.as_str().map(String::from)?;
            let raw = row.get("value")?.as_str()?;
            let value = serde_json::from_str(raw).ok()?;
            Some(UserSetting { key, value })
        })
        .collect();

    Response::from_json(&settings)
}

#[instrument(skip_all, fields(action = "update_instance_setting"))]
pub async fn update_setting(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let key = require_param(&ctx, "key")?;
    let d1 = ctx.env.d1("DB")?;
    if let Some(r) = require_admin(&d1, &user_id).await? {
        return Ok(r);
    }

    let body: UpsertSettingRequest = req.json().await?;
    let value_str = body.value.to_string();

    d1.prepare(
        "INSERT OR REPLACE INTO instance_settings (key, value, updated_at) \
         VALUES (?1, ?2, datetime('now'))",
    )
    .bind(&[key.as_str().into(), value_str.as_str().into()])?
    .run()
    .await?;

    Response::from_json(&UserSetting {
        key,
        value: body.value,
    })
}

// ── Invitation Codes ───────────────────────────────────────────────────────

#[instrument(skip_all)]
pub async fn list_codes(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let d1 = ctx.env.d1("DB")?;
    if let Some(r) = require_admin(&d1, &user_id).await? {
        return Ok(r);
    }

    let rows = d1
        .prepare(
            "SELECT id, code, created_by, used_by, reserved_by_email, reserved_until, \
                    created_at, used_at, expires_at \
             FROM invitation_codes ORDER BY created_at DESC",
        )
        .all()
        .await?
        .results::<serde_json::Value>()?;

    let codes: Vec<InvitationCode> = rows
        .into_iter()
        .filter_map(|row| {
            Some(InvitationCode {
                id: row.get("id")?.as_str().map(String::from)?,
                code: row.get("code")?.as_str().map(String::from)?,
                created_by: row.get("created_by")?.as_str().map(String::from)?,
                used_by: row
                    .get("used_by")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                reserved_by_email: row
                    .get("reserved_by_email")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                reserved_until: row
                    .get("reserved_until")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                created_at: row.get("created_at")?.as_str().map(String::from)?,
                used_at: row
                    .get("used_at")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                expires_at: row
                    .get("expires_at")
                    .and_then(|v| v.as_str())
                    .map(String::from),
            })
        })
        .collect();

    Response::from_json(&codes)
}

#[instrument(skip_all, fields(action = "create_invitation_code"))]
pub async fn create_code(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let d1 = ctx.env.d1("DB")?;
    if let Some(r) = require_admin(&d1, &user_id).await? {
        return Ok(r);
    }

    let body: CreateInvitationCodeRequest = req.json().await?;
    let id = Uuid::new_v4().to_string();
    // Generate an 8-character uppercase alphanumeric code from a UUID
    let code = Uuid::new_v4()
        .to_string()
        .replace('-', "")
        .to_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect::<String>();

    d1.prepare(
        "INSERT INTO invitation_codes (id, code, created_by, expires_at) \
         VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(&[
        id.as_str().into(),
        code.as_str().into(),
        user_id.as_str().into(),
        opt_str_to_js(&body.expires_at),
    ])?
    .run()
    .await?;

    Response::from_json(&InvitationCode {
        id,
        code,
        created_by: user_id,
        used_by: None,
        reserved_by_email: None,
        reserved_until: None,
        created_at: String::new(), // DB sets this; not read back here
        used_at: None,
        expires_at: body.expires_at,
    })
}

#[instrument(skip_all, fields(action = "delete_invitation_code"))]
pub async fn delete_code(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let id = require_param(&ctx, "id")?;
    let d1 = ctx.env.d1("DB")?;
    if let Some(r) = require_admin(&d1, &user_id).await? {
        return Ok(r);
    }

    d1.prepare("DELETE FROM invitation_codes WHERE id = ?1")
        .bind(&[id.into()])?
        .run()
        .await?;

    Ok(Response::empty()?.with_status(204))
}
