use worker::*;

/// Returns the dev bypass user_id if `DEV_AUTH_USER_ID` Worker var is set.
/// Used only in local dev — never set in prod/dev deployments.
pub fn dev_bypass_user_id(env: &Env) -> Option<String> {
    env.var("DEV_AUTH_USER_ID")
        .ok()
        .map(|v| v.to_string())
        .filter(|s| !s.is_empty())
}

/// Extracts user_id from X-User-Id header set by Gateway Worker.
/// In production, the API Worker is only reachable via service binding
/// from the Gateway, which validates auth and injects this header.
pub fn user_id_from_gateway(req: &Request) -> Result<String> {
    req.headers()
        .get("X-User-Id")?
        .ok_or_else(|| Error::from("Missing X-User-Id header"))
}

/// Extracts user email from X-User-Email header set by Gateway Worker.
/// Returns None if header is absent (e.g. in dev bypass mode).
pub fn user_email_from_gateway(req: &Request) -> Option<String> {
    req.headers()
        .get("X-User-Email")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
}
