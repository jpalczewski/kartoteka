use kartoteka_shared::types::{CreateListRequest, List};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    crate::server_fns::home::domain_list_to_shared, axum_login::AuthSession,
    kartoteka_auth::KartotekaBackend, kartoteka_domain as domain, sqlx::SqlitePool,
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

/// Rename a list (name required, description optional — pass empty string to clear).
#[server(prefix = "/leptos")]
pub async fn rename_list(
    id: String,
    name: String,
    description: Option<String>,
) -> Result<List, ServerFnError> {
    if name.trim().is_empty() {
        return Err(ServerFnError::new("name cannot be empty".to_string()));
    }
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let req = domain::lists::UpdateListRequest {
        name: Some(name),
        icon: None,
        description: description.map(|d| if d.trim().is_empty() { None } else { Some(d) }),
        list_type: None,
    };
    let list = domain::lists::update(&pool, &id, &user.id, &req)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("list not found".to_string()))?;
    Ok(domain_list_to_shared(list))
}

/// Toggle pinned state. Returns updated list, or None if not found.
#[server(prefix = "/leptos")]
pub async fn pin_list(id: String) -> Result<Option<List>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let result = domain::lists::toggle_pin(&pool, &id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(result.map(domain_list_to_shared))
}

/// Mark all items in a list as not completed.
#[server(prefix = "/leptos")]
pub async fn reset_list(id: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::lists::reset(&pool, &id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}
