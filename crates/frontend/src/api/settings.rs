pub async fn fetch_settings() -> Result<serde_json::Value, String> {
    super::get(&format!("{}/settings", super::API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn upsert_setting(key: &str, value: serde_json::Value) -> Result<(), String> {
    let json =
        serde_json::to_string(&serde_json::json!({ "value": value })).map_err(|e| e.to_string())?;
    let resp = gloo_net::http::Request::put(&format!("{}/settings/{key}", super::API_BASE))
        .headers(super::auth_headers())
        .credentials(web_sys::RequestCredentials::Include)
        .body(json)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() >= 400 {
        return Err(format!("HTTP {}", resp.status()));
    }
    Ok(())
}
