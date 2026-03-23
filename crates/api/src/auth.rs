use serde::Deserialize;
use worker::*;

const HANKO_API_URL: &str = env!("HANKO_API_URL");

#[derive(Deserialize)]
struct SessionValidation {
    is_valid: bool,
    user_id: Option<String>,
}

/// Validates Hanko session token from Authorization header.
/// Calls Hanko /sessions/validate endpoint.
/// Returns user_id on success.
pub async fn validate_session(req: &Request) -> Result<String> {
    let auth_header = req
        .headers()
        .get("Authorization")?
        .ok_or_else(|| Error::from("Missing Authorization header"))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| Error::from("Invalid Authorization format"))?;

    let url = format!("{HANKO_API_URL}/sessions/validate");
    let body = serde_json::json!({ "session_token": token });

    let mut init = RequestInit::new();
    init.with_method(Method::Post);
    init.with_body(Some(wasm_bindgen::JsValue::from_str(
        &serde_json::to_string(&body).map_err(|e| Error::from(e.to_string()))?,
    )));

    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    init.with_headers(headers);

    let req = Request::new_with_init(&url, &init)?;
    let mut resp = Fetch::Request(req).send().await?;

    if resp.status_code() != 200 {
        return Err(Error::from("Invalid session"));
    }

    let validation: SessionValidation = resp.json().await?;

    if !validation.is_valid {
        return Err(Error::from("Session expired"));
    }

    validation
        .user_id
        .ok_or_else(|| Error::from("No user_id in session"))
}
