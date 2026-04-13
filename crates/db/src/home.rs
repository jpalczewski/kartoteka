use crate::types::ContainerRow;
use crate::{DbError, SqlitePool, containers, lists};

/// Full parallel home data: containers + lists.
pub struct FullHomeData {
    pub pinned_containers: Vec<ContainerRow>,
    pub recent_containers: Vec<ContainerRow>,
    pub root_containers: Vec<ContainerRow>,
    pub pinned_lists: Vec<crate::lists::ListRow>,
    pub recent_lists: Vec<crate::lists::ListRow>,
    pub root_lists: Vec<crate::lists::ListRow>,
}

/// Fetch all home data in parallel via tokio::join!.
/// WAL mode allows concurrent SQLite readers.
#[tracing::instrument(skip(pool))]
pub async fn query(pool: &SqlitePool, user_id: &str) -> Result<FullHomeData, DbError> {
    let (pinned_c, recent_c, root_c, pinned_l, recent_l, root_l) = tokio::join!(
        containers::pinned(pool, user_id),
        containers::recent(pool, user_id),
        containers::root(pool, user_id),
        lists::pinned(pool, user_id),
        lists::recent(pool, user_id, 5),
        lists::root(pool, user_id),
    );
    Ok(FullHomeData {
        pinned_containers: pinned_c?,
        recent_containers: recent_c?,
        root_containers: root_c?,
        pinned_lists: pinned_l?,
        recent_lists: recent_l?,
        root_lists: root_l?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_user, test_pool};

    #[tokio::test]
    async fn home_query_returns_empty_for_new_user() {
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
}
