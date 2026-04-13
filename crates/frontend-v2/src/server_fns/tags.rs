use kartoteka_shared::types::{ListTagLink, Tag};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    axum_login::AuthSession,
    kartoteka_auth::KartotekaBackend,
    kartoteka_db as db,
    kartoteka_domain as domain,
    sqlx::SqlitePool,
    crate::server_fns::home::domain_tag_to_shared,
};

/// All tags for the current user (for tag filter bar and tag selectors).
#[server(prefix = "/leptos")]
pub async fn get_all_tags() -> Result<Vec<Tag>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let tags = domain::tags::list_all(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(tags.into_iter().map(domain_tag_to_shared).collect())
}

/// All list-tag associations for the current user.
#[server(prefix = "/leptos")]
pub async fn get_list_tag_links() -> Result<Vec<ListTagLink>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let pairs = db::tags::get_all_list_tag_links(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(pairs
        .into_iter()
        .map(|(list_id, tag_id)| ListTagLink { list_id, tag_id })
        .collect())
}

/// Assign a tag to a list (idempotent).
#[server(prefix = "/leptos")]
pub async fn assign_tag_to_list(list_id: String, tag_id: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::tags::assign_to_list(&pool, &user.id, &list_id, &tag_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Remove a tag from a list.
#[server(prefix = "/leptos")]
pub async fn remove_tag_from_list(list_id: String, tag_id: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::tags::remove_from_list(&pool, &user.id, &list_id, &tag_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}
