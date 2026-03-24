use gloo_net::http::{Headers, Request};
use kartoteka_shared::*;
use wasm_bindgen::JsCast;
use web_sys::js_sys;

const API_BASE: &str = env!("API_BASE_URL");

fn get_hanko_token() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok()??;
    storage.get_item("hanko_token").ok()?
}

fn auth_headers() -> Headers {
    let headers = Headers::new();
    headers.set("Content-Type", "application/json");
    if let Some(token) = get_hanko_token() {
        headers.set("Authorization", &format!("Bearer {token}"));
    }
    headers
}

fn get(url: &str) -> gloo_net::http::RequestBuilder {
    Request::get(url).headers(auth_headers())
}

fn del(url: &str) -> gloo_net::http::RequestBuilder {
    Request::delete(url).headers(auth_headers())
}

async fn post_json<T: serde::de::DeserializeOwned>(
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

async fn put_json<T: serde::de::DeserializeOwned>(
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

pub fn is_logged_in() -> bool {
    get_hanko_token().is_some()
}

pub fn get_user_email() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok()??;
    storage.get_item("hanko_user_email").ok()?
}

pub fn logout() {
    if let Some(window) = web_sys::window() {
        let hanko_logout = js_sys::Reflect::get(&window, &"__hankoLogout".into()).ok();
        if let Some(func) = hanko_logout {
            if let Ok(func) = func.dyn_into::<js_sys::Function>() {
                let _ = func.call0(&wasm_bindgen::JsValue::NULL);
            }
        }
    }
}

pub async fn fetch_lists() -> Result<Vec<List>, String> {
    get(&format!("{API_BASE}/lists"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn create_list(req: &CreateListRequest) -> Result<List, String> {
    post_json(&format!("{API_BASE}/lists"), req).await
}

pub async fn delete_list(id: &str) -> Result<(), String> {
    let resp = del(&format!("{API_BASE}/lists/{id}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.ok() {
        Ok(())
    } else {
        Err(format!("Błąd serwera: {}", resp.status()))
    }
}

pub async fn fetch_items(list_id: &str) -> Result<Vec<Item>, String> {
    get(&format!("{API_BASE}/lists/{list_id}/items"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn create_item(list_id: &str, req: &CreateItemRequest) -> Result<Item, String> {
    post_json(&format!("{API_BASE}/lists/{list_id}/items"), req).await
}

pub async fn update_item(
    list_id: &str,
    id: &str,
    req: &UpdateItemRequest,
) -> Result<Item, String> {
    put_json(&format!("{API_BASE}/lists/{list_id}/items/{id}"), req).await
}

pub async fn delete_item(list_id: &str, id: &str) -> Result<(), String> {
    del(&format!("{API_BASE}/lists/{list_id}/items/{id}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Tag CRUD ──────────────────────────────────────────────────

pub async fn fetch_tags() -> Result<Vec<Tag>, String> {
    get(&format!("{API_BASE}/tags"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn create_tag(req: &CreateTagRequest) -> Result<Tag, String> {
    post_json(&format!("{API_BASE}/tags"), req).await
}

pub async fn update_tag(id: &str, req: &UpdateTagRequest) -> Result<Tag, String> {
    put_json(&format!("{API_BASE}/tags/{id}"), req).await
}

pub async fn delete_tag(id: &str) -> Result<(), String> {
    del(&format!("{API_BASE}/tags/{id}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Tag assignments ───────────────────────────────────────────

pub async fn assign_tag_to_item(item_id: &str, tag_id: &str) -> Result<(), String> {
    let body = serde_json::to_string(&TagAssignment { tag_id: tag_id.to_string() })
        .map_err(|e| e.to_string())?;
    Request::post(&format!("{API_BASE}/items/{item_id}/tags"))
        .headers(auth_headers())
        .body(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn remove_tag_from_item(item_id: &str, tag_id: &str) -> Result<(), String> {
    del(&format!("{API_BASE}/items/{item_id}/tags/{tag_id}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn assign_tag_to_list(list_id: &str, tag_id: &str) -> Result<(), String> {
    let body = serde_json::to_string(&TagAssignment { tag_id: tag_id.to_string() })
        .map_err(|e| e.to_string())?;
    Request::post(&format!("{API_BASE}/lists/{list_id}/tags"))
        .headers(auth_headers())
        .body(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn remove_tag_from_list(list_id: &str, tag_id: &str) -> Result<(), String> {
    del(&format!("{API_BASE}/lists/{list_id}/tags/{tag_id}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Tag link bulk queries ─────────────────────────────────────

pub async fn fetch_list_tag_links() -> Result<Vec<ListTagLink>, String> {
    get(&format!("{API_BASE}/tag-links/lists"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn fetch_item_tag_links() -> Result<Vec<ItemTagLink>, String> {
    get(&format!("{API_BASE}/tag-links/items"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}
