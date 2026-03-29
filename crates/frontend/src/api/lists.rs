use kartoteka_shared::*;

pub async fn create_list(
    client: &impl super::HttpClient,
    req: &CreateListRequest,
) -> Result<List, super::ApiError> {
    super::api_post(client, &format!("{}/lists", super::API_BASE), req).await
}

pub async fn fetch_archived_lists(
    client: &impl super::HttpClient,
) -> Result<Vec<List>, super::ApiError> {
    super::api_get(client, &format!("{}/lists/archived", super::API_BASE)).await
}

pub async fn archive_list(
    client: &impl super::HttpClient,
    id: &str,
) -> Result<List, super::ApiError> {
    super::api_patch(
        client,
        &format!("{}/lists/{id}/archive", super::API_BASE),
        &serde_json::json!({}),
    )
    .await
}

pub async fn reset_list(client: &impl super::HttpClient, id: &str) -> Result<(), super::ApiError> {
    super::api_post_empty(
        client,
        &format!("{}/lists/{id}/reset", super::API_BASE),
        &serde_json::json!({}),
    )
    .await
}

pub async fn fetch_list(
    client: &impl super::HttpClient,
    id: &str,
) -> Result<List, super::ApiError> {
    super::api_get(client, &format!("{}/lists/{id}", super::API_BASE)).await
}

pub async fn update_list(
    client: &impl super::HttpClient,
    id: &str,
    req: &UpdateListRequest,
) -> Result<List, super::ApiError> {
    super::api_put(client, &format!("{}/lists/{id}", super::API_BASE), req).await
}

pub async fn delete_list(client: &impl super::HttpClient, id: &str) -> Result<(), super::ApiError> {
    super::api_delete(client, &format!("{}/lists/{id}", super::API_BASE)).await
}

pub async fn fetch_sublists(
    client: &impl super::HttpClient,
    parent_id: &str,
) -> Result<Vec<List>, super::ApiError> {
    super::api_get(
        client,
        &format!("{}/lists/{parent_id}/sublists", super::API_BASE),
    )
    .await
}

pub async fn create_sublist(
    client: &impl super::HttpClient,
    parent_id: &str,
    name: &str,
) -> Result<List, super::ApiError> {
    let body = serde_json::json!({ "name": name });
    super::api_post(
        client,
        &format!("{}/lists/{parent_id}/sublists", super::API_BASE),
        &body,
    )
    .await
}

pub async fn add_feature(
    client: &impl super::HttpClient,
    list_id: &str,
    feature_name: &str,
    config: serde_json::Value,
) -> Result<List, super::ApiError> {
    super::api_post(
        client,
        &format!(
            "{}/lists/{list_id}/features/{feature_name}",
            super::API_BASE
        ),
        &FeatureConfigRequest { config },
    )
    .await
}

/// Delete a feature from a list. The API returns the updated List.
pub async fn remove_feature(
    client: &impl super::HttpClient,
    list_id: &str,
    feature_name: &str,
) -> Result<List, super::ApiError> {
    let resp = client
        .request(
            super::Method::Delete,
            &format!(
                "{}/lists/{list_id}/features/{feature_name}",
                super::API_BASE
            ),
            None,
        )
        .await
        .map_err(super::ApiError::Network)?;
    super::parse_response(&resp)
}
