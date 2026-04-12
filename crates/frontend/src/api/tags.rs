use kartoteka_shared::*;

pub async fn fetch_tags_page(
    client: &impl super::HttpClient,
) -> Result<CursorPage<Tag>, super::ApiError> {
    super::api_get(client, &format!("{}/tags", super::API_BASE)).await
}

pub async fn fetch_tags(client: &impl super::HttpClient) -> Result<Vec<Tag>, super::ApiError> {
    let page = fetch_tags_page(client).await?;
    super::collect_all_pages(client, page).await
}

pub async fn create_tag(
    client: &impl super::HttpClient,
    req: &CreateTagRequest,
) -> Result<Tag, super::ApiError> {
    super::api_post(client, &format!("{}/tags", super::API_BASE), req).await
}

pub async fn update_tag(
    client: &impl super::HttpClient,
    id: &str,
    req: &UpdateTagRequest,
) -> Result<Tag, super::ApiError> {
    super::api_put(client, &format!("{}/tags/{id}", super::API_BASE), req).await
}

pub async fn delete_tag(client: &impl super::HttpClient, id: &str) -> Result<(), super::ApiError> {
    super::api_delete(client, &format!("{}/tags/{id}", super::API_BASE)).await
}

pub async fn merge_tag(
    client: &impl super::HttpClient,
    source_id: &str,
    target_id: &str,
) -> Result<Tag, super::ApiError> {
    let url = format!("{}/tags/{source_id}/merge", super::API_BASE);
    let body = serde_json::json!({ "target_tag_id": target_id });
    super::api_post(client, &url, &body).await
}

pub async fn fetch_tag_items(
    client: &impl super::HttpClient,
    tag_id: &str,
    recursive: bool,
) -> Result<Vec<DateItem>, super::ApiError> {
    let url = format!(
        "{}/tags/{tag_id}/items?recursive={recursive}",
        super::API_BASE
    );
    super::api_get(client, &url).await
}

pub async fn fetch_tag_entities(
    client: &impl super::HttpClient,
    tag_id: &str,
    recursive: bool,
    entity_type: Option<&str>,
) -> Result<Vec<SearchEntityResult>, super::ApiError> {
    let mut url = format!(
        "{}/tags/{tag_id}/entities?recursive={recursive}",
        super::API_BASE
    );
    if let Some(entity_type) = entity_type {
        url.push_str("&entity_type=");
        url.push_str(&super::encode_query_component(entity_type));
    }
    super::api_get(client, &url).await
}

pub async fn assign_tag_to_item(
    client: &impl super::HttpClient,
    item_id: &str,
    tag_id: &str,
) -> Result<(), super::ApiError> {
    let body = TagAssignment {
        tag_id: tag_id.to_string(),
    };
    super::api_post_empty(
        client,
        &format!("{}/items/{item_id}/tags", super::API_BASE),
        &body,
    )
    .await
}

pub async fn remove_tag_from_item(
    client: &impl super::HttpClient,
    item_id: &str,
    tag_id: &str,
) -> Result<(), super::ApiError> {
    super::api_delete(
        client,
        &format!("{}/items/{item_id}/tags/{tag_id}", super::API_BASE),
    )
    .await
}

pub async fn assign_tag_to_list(
    client: &impl super::HttpClient,
    list_id: &str,
    tag_id: &str,
) -> Result<(), super::ApiError> {
    let body = TagAssignment {
        tag_id: tag_id.to_string(),
    };
    super::api_post_empty(
        client,
        &format!("{}/lists/{list_id}/tags", super::API_BASE),
        &body,
    )
    .await
}

pub async fn remove_tag_from_list(
    client: &impl super::HttpClient,
    list_id: &str,
    tag_id: &str,
) -> Result<(), super::ApiError> {
    super::api_delete(
        client,
        &format!("{}/lists/{list_id}/tags/{tag_id}", super::API_BASE),
    )
    .await
}

pub async fn fetch_list_tag_links(
    client: &impl super::HttpClient,
) -> Result<Vec<ListTagLink>, super::ApiError> {
    super::api_get(client, &format!("{}/tag-links/lists", super::API_BASE)).await
}

pub async fn fetch_item_tag_links(
    client: &impl super::HttpClient,
) -> Result<Vec<ItemTagLink>, super::ApiError> {
    super::api_get(client, &format!("{}/tag-links/items", super::API_BASE)).await
}
