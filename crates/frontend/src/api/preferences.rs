use gloo_net::http::Request;
use serde::{Deserialize, Serialize};

use super::{API_BASE, auth_headers};

#[derive(Deserialize)]
pub struct PreferencesResponse {
    pub locale: String,
}

#[derive(Serialize)]
pub struct UpdatePreferencesBody {
    pub locale: String,
}

pub async fn get_preferences() -> Result<PreferencesResponse, String> {
    let url = format!("{API_BASE}/preferences");
    let resp = super::get(&url).send().await.map_err(|e| e.to_string())?;

    if resp.status() == 401 {
        return Err("unauthorized".to_string());
    }

    resp.json::<PreferencesResponse>()
        .await
        .map_err(|e| e.to_string())
}

pub async fn put_preferences(locale: &str) -> Result<(), String> {
    let body = UpdatePreferencesBody {
        locale: locale.to_string(),
    };
    let json = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    let resp = Request::put(&format!("{API_BASE}/preferences"))
        .headers(auth_headers())
        .credentials(web_sys::RequestCredentials::Include)
        .body(json)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status() >= 400 {
        return Err(format!("HTTP error {}", resp.status()));
    }

    Ok(())
}
