use gloo_net::http::Request;
use kartoteka_shared::*;

const API_BASE: &str = "/api";

pub async fn fetch_lists() -> Result<Vec<List>, String> {
    Request::get(&format!("{API_BASE}/lists"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn create_list(req: &CreateListRequest) -> Result<List, String> {
    Request::post(&format!("{API_BASE}/lists"))
        .json(req)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_list(id: &str) -> Result<(), String> {
    Request::delete(&format!("{API_BASE}/lists/{id}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn fetch_items(list_id: &str) -> Result<Vec<Item>, String> {
    Request::get(&format!("{API_BASE}/lists/{list_id}/items"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn create_item(list_id: &str, req: &CreateItemRequest) -> Result<Item, String> {
    Request::post(&format!("{API_BASE}/lists/{list_id}/items"))
        .json(req)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn update_item(list_id: &str, id: &str, req: &UpdateItemRequest) -> Result<Item, String> {
    Request::put(&format!("{API_BASE}/lists/{list_id}/items/{id}"))
        .json(req)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_item(list_id: &str, id: &str) -> Result<(), String> {
    Request::delete(&format!("{API_BASE}/lists/{list_id}/items/{id}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
