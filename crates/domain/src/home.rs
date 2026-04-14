use crate::DomainError;
use kartoteka_db::{SqlitePool, home as db_home};
use kartoteka_shared::types::{Container, HomeData, List, ListFeature};

fn row_to_container(r: kartoteka_db::types::ContainerRow) -> Container {
    Container {
        id: r.id,
        user_id: r.user_id,
        name: r.name,
        icon: r.icon,
        description: r.description,
        status: r.status,
        parent_container_id: r.parent_container_id,
        position: r.position,
        pinned: r.pinned,
        last_opened_at: r.last_opened_at,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn row_to_list(r: kartoteka_db::lists::ListRow) -> Result<List, DomainError> {
    let features: Vec<ListFeature> =
        serde_json::from_str(&r.features_json).map_err(|e| DomainError::Internal(e.to_string()))?;
    Ok(List {
        id: r.id,
        user_id: r.user_id,
        name: r.name,
        icon: r.icon,
        description: r.description,
        list_type: r.list_type,
        parent_list_id: r.parent_list_id,
        position: r.position,
        archived: r.archived != 0,
        container_id: r.container_id,
        pinned: r.pinned != 0,
        last_opened_at: r.last_opened_at,
        created_at: r.created_at,
        updated_at: r.updated_at,
        features,
    })
}

/// Fetch home page data: all six sections in parallel.
#[tracing::instrument(skip(pool))]
pub async fn query(pool: &SqlitePool, user_id: &str) -> Result<HomeData, DomainError> {
    let data = db_home::query(pool, user_id).await?;
    let pinned_lists: Result<Vec<List>, _> =
        data.pinned_lists.into_iter().map(row_to_list).collect();
    let recent_lists: Result<Vec<List>, _> =
        data.recent_lists.into_iter().map(row_to_list).collect();
    let root_lists: Result<Vec<List>, _> = data.root_lists.into_iter().map(row_to_list).collect();
    Ok(HomeData {
        pinned_containers: data
            .pinned_containers
            .into_iter()
            .map(row_to_container)
            .collect(),
        recent_containers: data
            .recent_containers
            .into_iter()
            .map(row_to_container)
            .collect(),
        root_containers: data
            .root_containers
            .into_iter()
            .map(row_to_container)
            .collect(),
        pinned_lists: pinned_lists?,
        recent_lists: recent_lists?,
        root_lists: root_lists?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{create_test_user, test_pool};
    use kartoteka_shared::types::CreateContainerRequest;

    #[tokio::test]
    async fn home_empty_for_new_user() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let data = query(&pool, &uid).await.unwrap();
        assert!(data.pinned_containers.is_empty());
        assert!(data.recent_containers.is_empty());
        assert!(data.root_containers.is_empty());
        assert!(data.pinned_lists.is_empty());
        assert!(data.recent_lists.is_empty());
        assert!(data.root_lists.is_empty());
    }

    #[tokio::test]
    async fn home_includes_root_containers() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        crate::containers::create(
            &pool,
            &uid,
            &CreateContainerRequest {
                name: "Root".into(),
                icon: None,
                description: None,
                status: None,
                parent_container_id: None,
            },
        )
        .await
        .unwrap();
        let data = query(&pool, &uid).await.unwrap();
        assert_eq!(data.root_containers.len(), 1);
    }
}
