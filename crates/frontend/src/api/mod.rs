mod containers;
mod items;
mod lists;
mod tags;

pub use containers::*;
pub use items::*;
pub use lists::*;
pub use tags::*;

use gloo_net::http::{Headers, Request};
use wasm_bindgen::JsCast;
use web_sys::js_sys;

pub(crate) const API_BASE: &str = env!("API_BASE_URL");

fn get_hanko_token() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok()??;
    storage.get_item("hanko_token").ok()?
}

pub(crate) fn auth_headers() -> Headers {
    let headers = Headers::new();
    headers.set("Content-Type", "application/json");
    if let Some(token) = get_hanko_token() {
        headers.set("Authorization", &format!("Bearer {token}"));
    }
    headers
}

pub(crate) fn get(url: &str) -> gloo_net::http::RequestBuilder {
    Request::get(url).headers(auth_headers())
}

pub(crate) fn del(url: &str) -> gloo_net::http::RequestBuilder {
    Request::delete(url).headers(auth_headers())
}

pub(crate) async fn post_json<T: serde::de::DeserializeOwned>(
    url: &str,
    body: &impl serde::Serialize,
) -> Result<T, String> {
    let json = serde_json::to_string(body).map_err(|e| e.to_string())?;
    Request::post(url)
        .headers(auth_headers())
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

pub fn is_logged_in() -> bool {
    get_hanko_token().is_some()
}

pub fn get_user_email() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok()??;
    storage.get_item("hanko_user_email").ok()?
}

pub fn logout() {
    if let Some(func) = web_sys::window().and_then(|w| {
        js_sys::Reflect::get(&w, &"__hankoLogout".into())
            .ok()
            .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
    }) {
        let _ = func.call0(&wasm_bindgen::JsValue::NULL);
    }
}
