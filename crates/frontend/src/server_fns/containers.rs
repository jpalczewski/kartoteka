#[cfg(not(feature = "ssr"))]
use kartoteka_shared::types::{Container, ContainerData, ContainerOption, CreateContainerRequest};
#[cfg(feature = "ssr")]
use kartoteka_shared::types::{
    Container, ContainerData, ContainerOption, CreateContainerRequest, List, UpdateContainerRequest,
};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use super::utils::build_ancestors;
#[cfg(feature = "ssr")]
use {
    crate::server_fns::home::domain_list_to_shared, axum_login::AuthSession,
    kartoteka_auth::KartotekaBackend, kartoteka_db, kartoteka_domain as domain, sqlx::SqlitePool,
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
pub async fn archive_container(id: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::containers::toggle_archive(&pool, &id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("not found".to_string()))?;
    Ok(())
}

#[server(prefix = "/leptos")]
pub async fn rename_container(
    id: String,
    name: String,
    description: Option<String>,
) -> Result<Container, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let req = UpdateContainerRequest {
        name: Some(name),
        description: Some(description),
        icon: None,
        status: None,
        location_id: None,
    };
    domain::containers::update(&pool, &id, &user.id, &req)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Rewrite container positions among siblings under `parent_container_id`.
#[server(prefix = "/leptos")]
pub async fn reorder_containers(
    parent_container_id: Option<String>,
    container_ids: Vec<String>,
) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    for (pos, id) in container_ids.iter().enumerate() {
        kartoteka_db::containers::move_container(
            &pool,
            id,
            &user.id,
            parent_container_id.as_deref(),
            pos as i32,
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    }
    Ok(())
}

/// Move a container under a new parent (or to root if `parent_container_id` is None).
/// Server computes next_position at the destination.
#[server(prefix = "/leptos")]
pub async fn move_container(
    id: String,
    parent_container_id: Option<String>,
) -> Result<Container, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let req = kartoteka_shared::types::MoveContainerRequest {
        parent_container_id,
        position: None,
    };
    domain::containers::move_container(&pool, &id, &user.id, &req)
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

    let (all_lists_res, all_containers_res) = tokio::join!(
        domain::lists::list_all(&pool, &user.id),
        domain::containers::list_all(&pool, &user.id),
    );
    let all_lists = all_lists_res.map_err(|e| ServerFnError::new(e.to_string()))?;
    let all_containers = all_containers_res.map_err(|e| ServerFnError::new(e.to_string()))?;

    let lists: Vec<List> = all_lists
        .into_iter()
        .filter(|l| l.container_id.as_deref() == Some(&container_id))
        .map(domain_list_to_shared)
        .collect();

    let ancestors = build_ancestors(&container, &all_containers);

    Ok(ContainerData {
        container,
        lists,
        children,
        ancestors,
    })
}

/// Returns containers available as move targets, with pre-built path labels.
///
/// - `exclude_subtree_of`: exclude this container and all its descendants (used when moving a
///   container to avoid self-reference or cycles).
/// - `folders_only`: when true, only return containers with `status IS NULL` (folders). Containers
///   with a status are projects and cannot be parents of other containers per hierarchy rules.
#[server(prefix = "/leptos")]
pub async fn get_containers_for_move(
    exclude_subtree_of: Option<String>,
    folders_only: bool,
) -> Result<Vec<ContainerOption>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;

    let all = domain::containers::list_all(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // BFS to collect the excluded subtree (self + all descendants).
    let excluded: std::collections::HashSet<String> = if let Some(ref root_id) = exclude_subtree_of
    {
        let mut set = std::collections::HashSet::new();
        set.insert(root_id.clone());
        let mut changed = true;
        while changed {
            changed = false;
            for c in &all {
                if !set.contains(&c.id) {
                    if let Some(ref pid) = c.parent_container_id {
                        if set.contains(pid) {
                            set.insert(c.id.clone());
                            changed = true;
                        }
                    }
                }
            }
        }
        set
    } else {
        std::collections::HashSet::new()
    };

    let mut options: Vec<ContainerOption> = all
        .iter()
        .filter(|c| !excluded.contains(&c.id))
        .filter(|c| !folders_only || c.status.is_none())
        .map(|c| {
            let path_label = build_path_label(c, &all);
            ContainerOption {
                id: c.id.clone(),
                path_label,
            }
        })
        .collect();

    options.sort_by(|a, b| a.path_label.cmp(&b.path_label));
    Ok(options)
}

#[cfg(feature = "ssr")]
fn build_path_label(container: &Container, all: &[Container]) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut current = container.parent_container_id.as_deref();
    for _ in 0..10 {
        let Some(pid) = current else { break };
        let Some(parent) = all.iter().find(|c| c.id == pid) else {
            break;
        };
        parts.push(parent.name.clone());
        current = parent.parent_container_id.as_deref();
    }
    parts.reverse();
    parts.push(container.name.clone());
    parts.join(" / ")
}
