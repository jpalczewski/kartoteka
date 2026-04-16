use kartoteka_shared::types::{Container, ContainerData, CreateContainerRequest, List};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    axum_login::AuthSession,
    kartoteka_auth::KartotekaBackend,
    kartoteka_domain as domain,
    sqlx::SqlitePool,
    crate::server_fns::home::domain_list_to_shared,
};

#[server(prefix = "/leptos")]
pub async fn create_container(req: CreateContainerRequest) -> Result<Container, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::containers::create(&pool, &user.id, &req)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(prefix = "/leptos")]
pub async fn delete_container(id: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::containers::delete(&pool, &id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(prefix = "/leptos")]
pub async fn toggle_container_pin(id: String) -> Result<Container, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::containers::toggle_pin(&pool, &id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Fetch container header + its direct lists + its direct child containers.
#[server(prefix = "/leptos")]
pub async fn get_container_data(container_id: String) -> Result<ContainerData, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;

    let container = domain::containers::get_one(&pool, &container_id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let children = domain::containers::get_children(&pool, &container_id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let all_lists = domain::lists::list_all(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let lists: Vec<List> = all_lists
        .into_iter()
        .filter(|l| l.container_id.as_deref() == Some(&container_id))
        .map(domain_list_to_shared)
        .collect();

    Ok(ContainerData {
        container,
        lists,
        children,
    })
}
