use kartoteka_shared::types::{CreateListRequest, List};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    crate::server_fns::home::domain_list_to_shared, axum_login::AuthSession,
    kartoteka_auth::KartotekaBackend, kartoteka_db, kartoteka_domain as domain, sqlx::SqlitePool,
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

/// Replace all features for a list (validates against list_type).
#[server(prefix = "/leptos")]
pub async fn update_list_features(
    list_id: String,
    features: Vec<String>,
) -> Result<List, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::lists::set_features(
        &pool,
        &list_id,
        &user.id,
        &domain::lists::SetFeaturesRequest { features },
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .map(domain_list_to_shared)
    .ok_or_else(|| ServerFnError::new("list not found".to_string()))
}

/// Get feature names enabled for a list. Lightweight alternative to get_list_data.
#[server(prefix = "/leptos")]
pub async fn get_list_feature_names(list_id: String) -> Result<Vec<String>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    Ok(domain::lists::get_one(&pool, &list_id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .map(|l| l.features.iter().map(|f| f.feature_name.clone()).collect())
        .unwrap_or_default())
}

/// Flat list of all non-archived lists owned by the user. Used as move targets
/// in the "Move to…" item dropdown.
#[server(prefix = "/leptos")]
pub async fn get_all_lists() -> Result<Vec<List>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let lists = domain::lists::list_all(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(lists
        .into_iter()
        .filter(|l| !l.archived)
        .map(domain_list_to_shared)
        .collect())
}

/// Rewrite list positions among siblings. All `list_ids` must share the same
/// parent (caller responsibility). `container_id` / `parent_list_id` indicate
/// the scope — one of them may be `Some`, or both `None` for root.
#[server(prefix = "/leptos")]
pub async fn reorder_lists(
    container_id: Option<String>,
    parent_list_id: Option<String>,
    list_ids: Vec<String>,
) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    for (pos, id) in list_ids.iter().enumerate() {
        kartoteka_db::lists::move_list(
            &pool,
            id,
            &user.id,
            pos as i64,
            container_id.as_deref(),
            parent_list_id.as_deref(),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    }
    Ok(())
}

/// Move a list: set its container and/or parent list. Both None = root.
/// Server computes next_position for the new location.
#[server(prefix = "/leptos")]
pub async fn move_list(
    list_id: String,
    container_id: Option<String>,
    parent_list_id: Option<String>,
) -> Result<List, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let position = kartoteka_db::lists::next_position(
        &pool,
        &user.id,
        container_id.as_deref(),
        parent_list_id.as_deref(),
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    let req = domain::lists::MoveListRequest {
        position,
        container_id,
        parent_list_id,
    };
    let list = domain::lists::move_list(&pool, &list_id, &user.id, &req)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("list not found".to_string()))?;
    Ok(domain_list_to_shared(list))
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
