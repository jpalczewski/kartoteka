use kartoteka_shared::*;

pub async fn fetch_home() -> Result<HomeData, String> {
    super::get(&format!("{}/home", super::API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn fetch_containers() -> Result<Vec<Container>, String> {
    super::get(&format!("{}/containers", super::API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn create_container(req: &CreateContainerRequest) -> Result<Container, String> {
    super::post_json(&format!("{}/containers", super::API_BASE), req).await
}

pub async fn fetch_container(id: &str) -> Result<ContainerDetail, String> {
    super::get(&format!("{}/containers/{id}", super::API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn update_container(id: &str, req: &UpdateContainerRequest) -> Result<Container, String> {
    super::put_json(&format!("{}/containers/{id}", super::API_BASE), req).await
}

pub async fn delete_container(id: &str) -> Result<(), String> {
    let resp = super::del(&format!("{}/containers/{id}", super::API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.ok() {
        Ok(())
    } else {
        Err(format!("Błąd serwera: {}", resp.status()))
    }
}

pub async fn fetch_container_children(id: &str) -> Result<ContainerChildrenResponse, String> {
    super::get(&format!("{}/containers/{id}/children", super::API_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn toggle_container_pin(id: &str) -> Result<Container, String> {
    super::patch_json(
        &format!("{}/containers/{id}/pin", super::API_BASE),
        &serde_json::json!({}),
    )
    .await
}

pub async fn move_list_to_container(
    list_id: &str,
    container_id: Option<&str>,
) -> Result<List, String> {
    let body = MoveListRequest {
        container_id: container_id.map(|s| s.to_string()),
    };
    super::patch_json(
        &format!("{}/lists/{list_id}/container", super::API_BASE),
        &body,
    )
    .await
}
