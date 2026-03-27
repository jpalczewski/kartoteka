use gloo_net::http::Request;
use kartoteka_shared::*;

pub async fn create_list(req: &CreateListRequest) -> Result<List, String> {
    super::post_json(&format!("{}/lists", super::API_BASE), req).await
}

pub async fn fetch_archived_lists() -> Result<Vec<List>, String> {
    super::get(&format!("{}/lists/archived", super::API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn archive_list(id: &str) -> Result<List, String> {
    super::patch_json(
        &format!("{}/lists/{id}/archive", super::API_BASE),
        &serde_json::json!({}),
    )
    .await
}

pub async fn reset_list(id: &str) -> Result<(), String> {
    let json = serde_json::to_string(&serde_json::json!({})).map_err(|e| e.to_string())?;
    let resp = Request::post(&format!("{}/lists/{id}/reset", super::API_BASE))
        .headers(super::auth_headers())
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

pub async fn fetch_list(id: &str) -> Result<List, String> {
    super::get(&format!("{}/lists/{id}", super::API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn update_list(id: &str, req: &UpdateListRequest) -> Result<List, String> {
    super::put_json(&format!("{}/lists/{id}", super::API_BASE), req).await
}

pub async fn delete_list(id: &str) -> Result<(), String> {
    let resp = super::del(&format!("{}/lists/{id}", super::API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.ok() {
        Ok(())
    } else {
        Err(format!("Błąd serwera: {}", resp.status()))
    }
}

pub async fn fetch_sublists(parent_id: &str) -> Result<Vec<List>, String> {
    super::get(&format!("{}/lists/{parent_id}/sublists", super::API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn create_sublist(parent_id: &str, name: &str) -> Result<List, String> {
    let body = serde_json::json!({ "name": name });
    super::post_json(
        &format!("{}/lists/{parent_id}/sublists", super::API_BASE),
        &body,
    )
    .await
}

pub async fn add_feature(
    list_id: &str,
    feature_name: &str,
    config: serde_json::Value,
) -> Result<List, String> {
    super::post_json(
        &format!(
            "{}/lists/{list_id}/features/{feature_name}",
            super::API_BASE
        ),
        &FeatureConfigRequest { config },
    )
    .await
}

pub async fn remove_feature(list_id: &str, feature_name: &str) -> Result<List, String> {
    let resp = super::del(&format!(
        "{}/lists/{list_id}/features/{feature_name}",
        super::API_BASE
    ))
    .send()
    .await
    .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}
