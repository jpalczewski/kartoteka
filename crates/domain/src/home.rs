use crate::DomainError;
use kartoteka_db::{SqlitePool, home as db_home};
use kartoteka_shared::types::{Container, HomeData};

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

/// Fetch home page data. Container-only in B1; B2 adds lists.
#[tracing::instrument(skip(pool))]
pub async fn query(pool: &SqlitePool, user_id: &str) -> Result<HomeData, DomainError> {
    let data = db_home::query(pool, user_id).await?;
    Ok(HomeData {
        pinned_containers: data.pinned.into_iter().map(row_to_container).collect(),
        recent_containers: data.recent.into_iter().map(row_to_container).collect(),
        root_containers: data.root.into_iter().map(row_to_container).collect(),
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
        assert_eq!(data.root_containers[0].name, "Root");
    }
}
