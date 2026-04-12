use crate::{containers, DbError, SqlitePool};
use crate::types::ContainerRow;

/// Parallel home data returned by db::home::query.
/// Container-only in B1; B2 extends with list fields.
pub struct ContainerHomeData {
    pub pinned: Vec<ContainerRow>,
    pub recent: Vec<ContainerRow>,
    pub root: Vec<ContainerRow>,
}

/// Fetch all home container data in parallel via tokio::join!.
#[tracing::instrument(skip(pool))]
pub async fn query(pool: &SqlitePool, user_id: &str) -> Result<ContainerHomeData, DbError> {
    let (pinned, recent, root) = tokio::join!(
        containers::pinned(pool, user_id),
        containers::recent(pool, user_id),
        containers::root(pool, user_id),
    );
    Ok(ContainerHomeData {
        pinned: pinned?,
        recent: recent?,
        root: root?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_user, test_pool};
    use kartoteka_shared::types::CreateContainerRequest;

    #[tokio::test]
    async fn home_query_returns_empty_for_new_user() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let data = query(&pool, &uid).await.unwrap();
        assert!(data.pinned.is_empty());
        assert!(data.recent.is_empty());
        assert!(data.root.is_empty());
    }

    #[tokio::test]
    async fn home_query_returns_root_containers() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateContainerRequest {
            name: "Root".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
        };
        crate::containers::insert(&pool, &uid, &req, 0).await.unwrap();
        let data = query(&pool, &uid).await.unwrap();
        assert_eq!(data.root.len(), 1);
        assert_eq!(data.root[0].name, "Root");
    }

    #[tokio::test]
    async fn home_query_separates_pinned() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let req = CreateContainerRequest {
            name: "C".into(),
            icon: None,
            description: None,
            status: None,
            parent_container_id: None,
        };
        let c = crate::containers::insert(&pool, &uid, &req, 0).await.unwrap();
        crate::containers::toggle_pin(&pool, &c.id, &uid).await.unwrap();
        let data = query(&pool, &uid).await.unwrap();
        assert_eq!(data.pinned.len(), 1);
        assert!(data.pinned[0].pinned);
    }
}
