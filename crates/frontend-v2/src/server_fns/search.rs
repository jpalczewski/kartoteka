use kartoteka_shared::models::search::SearchEntityResult;
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    axum_login::AuthSession, kartoteka_auth::KartotekaBackend, kartoteka_domain as domain,
    sqlx::SqlitePool,
};

#[server(prefix = "/leptos")]
pub async fn search_items(query: String) -> Result<Vec<SearchEntityResult>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;

    domain::search::search(&pool, &user.id, &query)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
