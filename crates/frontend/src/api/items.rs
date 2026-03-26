use kartoteka_shared::*;

pub async fn fetch_items(list_id: &str) -> Result<Vec<Item>, String> {
    super::get(&format!("{}/lists/{list_id}/items", super::API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn create_item(list_id: &str, req: &CreateItemRequest) -> Result<Item, String> {
    super::post_json(&format!("{}/lists/{list_id}/items", super::API_BASE), req).await
}

pub async fn update_item(list_id: &str, id: &str, req: &UpdateItemRequest) -> Result<Item, String> {
    super::put_json(
        &format!("{}/lists/{list_id}/items/{id}", super::API_BASE),
        req,
    )
    .await
}

pub async fn delete_item(list_id: &str, id: &str) -> Result<(), String> {
    super::del(&format!("{}/lists/{list_id}/items/{id}", super::API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn move_item(item_id: &str, target_list_id: &str) -> Result<Item, String> {
    let body = serde_json::json!({ "target_list_id": target_list_id });
    super::patch_json(&format!("{}/items/{item_id}/move", super::API_BASE), &body).await
}

pub async fn fetch_items_by_date(
    date: &str,
    include_overdue: bool,
) -> Result<Vec<DateItem>, String> {
    super::get(&format!(
        "{}/items/by-date?date={}&include_overdue={}",
        super::API_BASE,
        date,
        include_overdue
    ))
    .send()
    .await
    .map_err(|e| e.to_string())?
    .json()
    .await
    .map_err(|e| e.to_string())
}
