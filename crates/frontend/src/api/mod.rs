mod containers;
mod items;
mod lists;
mod tags;

pub use containers::*;
pub use items::*;
pub use lists::*;
pub use tags::*;

use gloo_net::http::{Headers, Request};

pub(crate) const API_BASE: &str = env!("API_BASE_URL");

pub(crate) fn auth_headers() -> Headers {
    let headers = Headers::new();
    headers.set("Content-Type", "application/json");
    headers
}

pub(crate) fn get(url: &str) -> gloo_net::http::RequestBuilder {
    Request::get(url)
        .headers(auth_headers())
        .credentials(web_sys::RequestCredentials::Include)
}

pub(crate) fn del(url: &str) -> gloo_net::http::RequestBuilder {
    Request::delete(url)
        .headers(auth_headers())
        .credentials(web_sys::RequestCredentials::Include)
}

pub(crate) async fn post_json<T: serde::de::DeserializeOwned>(
    url: &str,
    body: &impl serde::Serialize,
) -> Result<T, String> {
    let json = serde_json::to_string(body).map_err(|e| e.to_string())?;
    Request::post(url)
        .headers(auth_headers())
        .credentials(web_sys::RequestCredentials::Include)
        .body(json)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub(crate) async fn put_json<T: serde::de::DeserializeOwned>(
    url: &str,
    body: &impl serde::Serialize,
) -> Result<T, String> {
    let json = serde_json::to_string(body).map_err(|e| e.to_string())?;
    Request::put(url)
        .headers(auth_headers())
        .credentials(web_sys::RequestCredentials::Include)
        .body(json)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub(crate) async fn patch_json<T: serde::de::DeserializeOwned>(
    url: &str,
    body: &impl serde::Serialize,
) -> Result<T, String> {
    let json = serde_json::to_string(body).map_err(|e| e.to_string())?;
    let resp = Request::patch(url)
        .headers(auth_headers())
        .credentials(web_sys::RequestCredentials::Include)
        .body(json)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() >= 400 {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

/// Auth base URL — same origin as the page (Trunk proxies /auth to Gateway)
pub fn auth_base() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default()
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SessionInfo {
    pub user: SessionUser,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[allow(dead_code)]
pub struct SessionUser {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
}

/// Check current session. Returns Some(SessionInfo) if logged in.
pub async fn get_session() -> Option<SessionInfo> {
    let url = format!("{}/auth/api/get-session", auth_base());
    let resp = Request::get(&url)
        .credentials(web_sys::RequestCredentials::Include)
        .send()
        .await
        .ok()?;
    if resp.status() == 200 {
        resp.json::<SessionInfo>().await.ok()
    } else {
        None
    }
}

/// Sign out and redirect to /login
pub fn logout() {
    wasm_bindgen_futures::spawn_local(async {
        let url = format!("{}/auth/api/sign-out", auth_base());
        let _ = Request::post(&url)
            .credentials(web_sys::RequestCredentials::Include)
            .send()
            .await;
        if let Some(window) = web_sys::window() {
            let _ = window.location().set_href("/login");
        }
    });
}
