pub async fn fetch_settings(
    client: &impl super::HttpClient,
) -> Result<serde_json::Value, super::ApiError> {
    super::api_get(client, &format!("{}/settings", super::API_BASE)).await
}

pub async fn upsert_setting(
    client: &impl super::HttpClient,
    key: &str,
    value: serde_json::Value,
) -> Result<(), super::ApiError> {
    super::api_put_empty(
        client,
        &format!("{}/settings/{key}", super::API_BASE),
        &serde_json::json!({ "value": value }),
    )
    .await
}
