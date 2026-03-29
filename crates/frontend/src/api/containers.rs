use kartoteka_shared::*;

pub async fn fetch_home(client: &impl super::HttpClient) -> Result<HomeData, super::ApiError> {
    super::api_get(client, &format!("{}/home", super::API_BASE)).await
}

pub async fn fetch_containers(
    client: &impl super::HttpClient,
) -> Result<Vec<Container>, super::ApiError> {
    super::api_get(client, &format!("{}/containers", super::API_BASE)).await
}

pub async fn create_container(
    client: &impl super::HttpClient,
    req: &CreateContainerRequest,
) -> Result<Container, super::ApiError> {
    super::api_post(client, &format!("{}/containers", super::API_BASE), req).await
}

pub async fn fetch_container(
    client: &impl super::HttpClient,
    id: &str,
) -> Result<ContainerDetail, super::ApiError> {
    super::api_get(client, &format!("{}/containers/{id}", super::API_BASE)).await
}

pub async fn update_container(
    client: &impl super::HttpClient,
    id: &str,
    req: &UpdateContainerRequest,
) -> Result<Container, super::ApiError> {
    super::api_put(client, &format!("{}/containers/{id}", super::API_BASE), req).await
}

pub async fn delete_container(
    client: &impl super::HttpClient,
    id: &str,
) -> Result<(), super::ApiError> {
    super::api_delete(client, &format!("{}/containers/{id}", super::API_BASE)).await
}

pub async fn fetch_container_children(
    client: &impl super::HttpClient,
    id: &str,
) -> Result<ContainerChildrenResponse, super::ApiError> {
    super::api_get(client, &format!("{}/containers/{id}/children", super::API_BASE)).await
}

pub async fn toggle_container_pin(
    client: &impl super::HttpClient,
    id: &str,
) -> Result<Container, super::ApiError> {
    super::api_patch(
        client,
        &format!("{}/containers/{id}/pin", super::API_BASE),
        &serde_json::json!({}),
    )
    .await
}

pub async fn move_list_to_container(
    client: &impl super::HttpClient,
    list_id: &str,
    container_id: Option<&str>,
) -> Result<List, super::ApiError> {
    let body = MoveListRequest {
        container_id: container_id.map(|s| s.to_string()),
    };
    super::api_patch(
        client,
        &format!("{}/lists/{list_id}/container", super::API_BASE),
        &body,
    )
    .await
}
