use kartoteka_shared::types::{CreateListRequest, List};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    axum_login::AuthSession,
    kartoteka_auth::KartotekaBackend,
    kartoteka_domain as domain,
    sqlx::SqlitePool,
    crate::server_fns::home::domain_list_to_shared,
};

/// Create a new list. `container_id` puts the list inside a container;
/// `parent_list_id` makes it a sublist. Both optional.
#[server(prefix = "/leptos")]
pub async fn create_list(req: CreateListRequest) -> Result<List, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let list = domain::lists::create(
        &pool,
        &user.id,
        &domain::lists::CreateListRequest {
            name: req.name,
            list_type: req.list_type,
            icon: req.icon,
            description: req.description,
            container_id: req.container_id,
            parent_list_id: req.parent_list_id,
            features: req.features,
        },
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(domain_list_to_shared(list))
}

/// Delete a list and all its items.
#[server(prefix = "/leptos")]
pub async fn delete_list(id: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::lists::delete(&pool, &id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

/// Toggle archived state of a list. Returns the updated list if found.
#[server(prefix = "/leptos")]
pub async fn archive_list(id: String) -> Result<Option<List>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let result = domain::lists::toggle_archive(&pool, &id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(result.map(domain_list_to_shared))
}
