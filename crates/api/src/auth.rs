use worker::*;

pub fn check_auth(req: &Request, env: &Env) -> Result<()> {
    let token = env.secret("AUTH_TOKEN")?.to_string();

    let auth_header = req
        .headers()
        .get("Authorization")?
        .ok_or_else(|| Error::from("Missing Authorization header"))?;

    let bearer = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| Error::from("Invalid Authorization format"))?;

    if bearer != token {
        return Err(Error::from("Invalid token"));
    }

    Ok(())
}
