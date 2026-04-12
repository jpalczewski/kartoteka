use kartoteka_shared::*;

pub async fn fetch_items_page(
    client: &impl super::HttpClient,
    list_id: &str,
) -> Result<CursorPage<Item>, super::ApiError> {
    super::api_get(
        client,
        &format!("{}/lists/{list_id}/items", super::API_BASE),
    )
    .await
}

pub async fn fetch_items(
    client: &impl super::HttpClient,
    list_id: &str,
) -> Result<Vec<Item>, super::ApiError> {
    let page = fetch_items_page(client, list_id).await?;
    super::collect_all_pages(client, page).await
}

pub async fn fetch_item_detail(
    client: &impl super::HttpClient,
    list_id: &str,
    item_id: &str,
) -> Result<ItemDetailResponse, super::ApiError> {
    super::api_get(
        client,
        &format!("{}/lists/{list_id}/items/{item_id}", super::API_BASE),
    )
    .await
}

pub async fn create_item(
    client: &impl super::HttpClient,
    list_id: &str,
    req: &CreateItemRequest,
) -> Result<Item, super::ApiError> {
    super::api_post(
        client,
        &format!("{}/lists/{list_id}/items", super::API_BASE),
        req,
    )
    .await
}

#[allow(dead_code)]
pub async fn create_items(
    client: &impl super::HttpClient,
    list_id: &str,
    req: &CreateItemsRequest,
) -> Result<Vec<Item>, super::ApiError> {
    super::api_post(
        client,
        &format!("{}/lists/{list_id}/items/batch", super::API_BASE),
        req,
    )
    .await
}

pub async fn reorder_items(
    client: &impl super::HttpClient,
    list_id: &str,
    req: &ReorderItemsRequest,
) -> Result<(), super::ApiError> {
    super::api_patch_empty(
        client,
        &format!("{}/lists/{list_id}/items/reorder", super::API_BASE),
        req,
    )
    .await
}

pub async fn update_item(
    client: &impl super::HttpClient,
    list_id: &str,
    id: &str,
    req: &UpdateItemRequest,
) -> Result<Item, super::ApiError> {
    super::api_put(
        client,
        &format!("{}/lists/{list_id}/items/{id}", super::API_BASE),
        req,
    )
    .await
}

pub async fn delete_item(
    client: &impl super::HttpClient,
    list_id: &str,
    id: &str,
) -> Result<(), super::ApiError> {
    super::api_delete(
        client,
        &format!("{}/lists/{list_id}/items/{id}", super::API_BASE),
    )
    .await
}

#[allow(dead_code)]
pub async fn move_item(
    client: &impl super::HttpClient,
    item_id: &str,
    target_list_id: &str,
) -> Result<Item, super::ApiError> {
    let body = MoveItemsRequest {
        item_ids: vec![item_id.to_string()],
        target_list_id: target_list_id.to_string(),
    };
    super::api_patch(
        client,
        &format!("{}/items/{item_id}/move", super::API_BASE),
        &body,
    )
    .await
}

#[allow(dead_code)]
pub async fn move_items(
    client: &impl super::HttpClient,
    req: &MoveItemsRequest,
) -> Result<Vec<Item>, super::ApiError> {
    super::api_patch(client, &format!("{}/items/move", super::API_BASE), req).await
}

pub async fn set_item_placement(
    client: &impl super::HttpClient,
    item_id: &str,
    req: &SetItemPlacementRequest,
) -> Result<Item, super::ApiError> {
    super::api_patch(
        client,
        &format!("{}/items/{item_id}/placement", super::API_BASE),
        req,
    )
    .await
}

#[allow(dead_code)]
pub async fn set_items_completed(
    client: &impl super::HttpClient,
    req: &SetItemsCompletedRequest,
) -> Result<Vec<Item>, super::ApiError> {
    super::api_patch(client, &format!("{}/items/completed", super::API_BASE), req).await
}

pub async fn fetch_calendar_counts(
    client: &impl super::HttpClient,
    from: &str,
    to: &str,
    date_field: &str,
) -> Result<Vec<DaySummary>, super::ApiError> {
    super::api_get(
        client,
        &format!(
            "{}/items/calendar?from={}&to={}&detail=counts&date_field={}",
            super::API_BASE,
            from,
            to,
            date_field
        ),
    )
    .await
}

pub async fn fetch_calendar_full(
    client: &impl super::HttpClient,
    from: &str,
    to: &str,
    date_field: &str,
) -> Result<Vec<DayItems>, super::ApiError> {
    super::api_get(
        client,
        &format!(
            "{}/items/calendar?from={}&to={}&detail=full&date_field={}",
            super::API_BASE,
            from,
            to,
            date_field
        ),
    )
    .await
}

pub async fn fetch_items_by_date(
    client: &impl super::HttpClient,
    date: &str,
    include_overdue: bool,
    date_field: &str,
) -> Result<Vec<DateItem>, super::ApiError> {
    super::api_get(
        client,
        &format!(
            "{}/items/by-date?date={}&include_overdue={}&date_field={}",
            super::API_BASE,
            date,
            include_overdue,
            date_field
        ),
    )
    .await
}

pub async fn search_items_page(
    client: &impl super::HttpClient,
    state: &crate::state::search_route::SearchRouteState,
) -> Result<CursorPage<SearchItemResult>, super::ApiError> {
    let mut params = Vec::new();

    if let Some(query) = &state.query {
        params.push(format!("query={}", super::encode_query_component(query)));
    }
    if !state.search_title {
        params.push("search_title=false".to_string());
    }
    if !state.search_description {
        params.push("search_description=false".to_string());
    }
    for tag_id in state.tag_ids.iter() {
        params.push(format!("tag_id={}", super::encode_query_component(tag_id)));
    }
    if let Some(completed) = state.completed.as_api_value() {
        params.push(format!("completed={completed}"));
    }
    if state.include_archived {
        params.push("include_archived=true".to_string());
    }

    let query = params.join("&");
    let url = if query.is_empty() {
        format!("{}/items/search", super::API_BASE)
    } else {
        format!("{}/items/search?{}", super::API_BASE, query)
    };

    super::api_get(client, &url).await
}
