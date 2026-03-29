use kartoteka_shared::{PreferencesResponse, UpdatePreferencesBody};

pub async fn get_preferences(
    client: &impl super::HttpClient,
) -> Result<PreferencesResponse, super::ApiError> {
    super::api_get(client, &format!("{}/preferences", super::API_BASE)).await
}

pub async fn put_preferences(
    client: &impl super::HttpClient,
    locale: &str,
) -> Result<(), super::ApiError> {
    let body = UpdatePreferencesBody {
        locale: locale.to_string(),
    };
    super::api_put_empty(client, &format!("{}/preferences", super::API_BASE), &body).await
}
