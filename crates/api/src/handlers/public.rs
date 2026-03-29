use kartoteka_shared::constants::INSTANCE_SETTING_REGISTRATION_MODE;
use kartoteka_shared::dto::requests::ValidateInviteRequest;
use kartoteka_shared::dto::responses::{RegistrationModeResponse, ValidateInviteResponse};
use worker::*;

/// GET /api/public/registration-mode — no auth required
pub async fn get_registration_mode(_req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let d1 = ctx.env.d1("DB")?;

    let row = d1
        .prepare("SELECT value FROM instance_settings WHERE key = ?1")
        .bind(&[INSTANCE_SETTING_REGISTRATION_MODE.into()])?
        .first::<serde_json::Value>(None)
        .await?;

    let mode = row
        .and_then(|v| v.get("value")?.as_str().map(String::from))
        .and_then(|s| serde_json::from_str::<String>(&s).ok())
        .unwrap_or_else(|| "open".to_string());

    Response::from_json(&RegistrationModeResponse { mode })
}

/// POST /api/public/validate-invite — no auth required
pub async fn validate_invite(mut req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let body: ValidateInviteRequest = req.json().await?;
    let d1 = ctx.env.d1("DB")?;

    // Check that the code exists, is unused, and not expired
    let existing = d1
        .prepare(
            "SELECT id, reserved_by_email, reserved_until \
             FROM invitation_codes \
             WHERE code = ?1 AND used_by IS NULL \
               AND (expires_at IS NULL OR expires_at > datetime('now'))",
        )
        .bind(&[body.code.as_str().into()])?
        .first::<serde_json::Value>(None)
        .await?;

    let Some(_row) = existing else {
        return Response::from_json(&ValidateInviteResponse {
            valid: false,
            error: Some("invalid_code".to_string()),
        });
    };

    // Row exists; attempt atomic reservation.
    // The UPDATE only succeeds if not currently reserved by someone else (or reservation expired).
    // Atomically try to reserve: only succeeds if not reserved or reservation expired
    d1.prepare(
        "UPDATE invitation_codes \
         SET reserved_by_email = ?1, reserved_until = datetime('now', '+10 minutes') \
         WHERE code = ?2 AND used_by IS NULL \
           AND (reserved_until IS NULL OR reserved_until < datetime('now'))",
    )
    .bind(&[body.email.as_str().into(), body.code.as_str().into()])?
    .run()
    .await?;

    // Confirm reservation belongs to this email
    let confirmed = d1
        .prepare(
            "SELECT id FROM invitation_codes \
             WHERE code = ?1 AND reserved_by_email = ?2 AND reserved_until > datetime('now')",
        )
        .bind(&[body.code.as_str().into(), body.email.as_str().into()])?
        .first::<serde_json::Value>(None)
        .await?;

    if confirmed.is_some() {
        Response::from_json(&ValidateInviteResponse {
            valid: true,
            error: None,
        })
    } else {
        Response::from_json(&ValidateInviteResponse {
            valid: false,
            error: Some("code_in_use".to_string()),
        })
    }
}
