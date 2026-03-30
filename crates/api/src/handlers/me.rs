use crate::auth::user_email_from_gateway;
use crate::helpers::ensure_user_exists;
use kartoteka_shared::MeResponse;
use tracing::instrument;
use worker::*;

#[instrument(skip_all)]
pub async fn get_me(req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let d1 = ctx.env.d1("DB")?;

    let email = user_email_from_gateway(&req).unwrap_or_default();
    let initial_admin_email = ctx
        .env
        .var("INITIAL_ADMIN_EMAIL")
        .ok()
        .map(|v| v.to_string())
        .unwrap_or_default();

    let is_admin = ensure_user_exists(&d1, &user_id, &email, &initial_admin_email).await?;

    // Optional: finalize an invite code after successful signup
    let url = req.url()?;
    if let Some(invite_code) = url
        .query_pairs()
        .find(|(k, _)| k == "invite_code")
        .map(|(_, v)| v.into_owned())
    {
        d1.prepare(
            "UPDATE invitation_codes \
             SET used_by = ?1, used_at = datetime('now'), \
                 reserved_by_email = NULL, reserved_until = NULL \
             WHERE code = ?2 AND used_by IS NULL AND reserved_by_email = ?3",
        )
        .bind(&[user_id.into(), invite_code.into(), email.as_str().into()])?
        .run()
        .await?;
    }

    Response::from_json(&MeResponse { is_admin })
}
