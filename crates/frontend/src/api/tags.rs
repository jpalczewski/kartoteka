use gloo_net::http::Request;
use kartoteka_shared::*;

pub async fn fetch_tags() -> Result<Vec<Tag>, String> {
    super::get(&format!("{}/tags", super::API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn create_tag(req: &CreateTagRequest) -> Result<Tag, String> {
    super::post_json(&format!("{}/tags", super::API_BASE), req).await
}

pub async fn update_tag(id: &str, req: &UpdateTagRequest) -> Result<Tag, String> {
    super::put_json(&format!("{}/tags/{id}", super::API_BASE), req).await
}

pub async fn delete_tag(id: &str) -> Result<(), String> {
    super::del(&format!("{}/tags/{id}", super::API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn merge_tag(source_id: &str, target_id: &str) -> Result<Tag, String> {
    let url = format!("{}/tags/{source_id}/merge", super::API_BASE);
    let body = serde_json::json!({ "target_tag_id": target_id });
    super::post_json(&url, &body).await
}

pub async fn fetch_tag_items(
    tag_id: &str,
    recursive: bool,
) -> Result<Vec<serde_json::Value>, String> {
    let url = format!(
        "{}/tags/{tag_id}/items?recursive={recursive}",
        super::API_BASE
    );
    super::get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn assign_tag_to_item(item_id: &str, tag_id: &str) -> Result<(), String> {
    let body = serde_json::to_string(&TagAssignment {
        tag_id: tag_id.to_string(),
    })
    .map_err(|e| e.to_string())?;
    Request::post(&format!("{}/items/{item_id}/tags", super::API_BASE))
        .headers(super::auth_headers())
        .body(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn remove_tag_from_item(item_id: &str, tag_id: &str) -> Result<(), String> {
    super::del(&format!(
        "{}/items/{item_id}/tags/{tag_id}",
        super::API_BASE
    ))
    .send()
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn assign_tag_to_list(list_id: &str, tag_id: &str) -> Result<(), String> {
    let body = serde_json::to_string(&TagAssignment {
        tag_id: tag_id.to_string(),
    })
    .map_err(|e| e.to_string())?;
    Request::post(&format!("{}/lists/{list_id}/tags", super::API_BASE))
        .headers(super::auth_headers())
        .body(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn remove_tag_from_list(list_id: &str, tag_id: &str) -> Result<(), String> {
    super::del(&format!(
        "{}/lists/{list_id}/tags/{tag_id}",
        super::API_BASE
    ))
    .send()
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn fetch_list_tag_links() -> Result<Vec<ListTagLink>, String> {
    super::get(&format!("{}/tag-links/lists", super::API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn fetch_item_tag_links() -> Result<Vec<ItemTagLink>, String> {
    super::get(&format!("{}/tag-links/items", super::API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}
