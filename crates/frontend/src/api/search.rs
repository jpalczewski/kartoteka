use kartoteka_shared::*;

pub async fn search_entities_page(
    client: &impl super::HttpClient,
    query: &str,
) -> Result<CursorPage<SearchEntityResult>, super::ApiError> {
    let encoded_query = super::encode_query_component(query);
    super::api_get(
        client,
        &format!("{}/search?query={encoded_query}", super::API_BASE),
    )
    .await
}
