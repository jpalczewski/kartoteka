use crate::DbError;
use sqlx::SqlitePool;

#[derive(Debug, sqlx::FromRow)]
pub struct SearchItemRow {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub list_id: String,
    pub list_name: String,
    pub updated_at: String,
    pub completed: bool,
}

/// Build a safe FTS5 prefix-match query from free-form user input.
/// Each whitespace-delimited token becomes `"token"*` so partial matches work.
/// Tokens containing only special characters are dropped.
fn fts5_prefix_query(query: &str) -> Option<String> {
    let terms: Vec<String> = query
        .split_whitespace()
        .map(|t| t.replace('"', ""))
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{t}\"*"))
        .collect();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

#[tracing::instrument(skip(pool))]
pub async fn search_items(
    pool: &SqlitePool,
    user_id: &str,
    query: &str,
) -> Result<Vec<SearchItemRow>, DbError> {
    let Some(fts_query) = fts5_prefix_query(query) else {
        return Ok(vec![]);
    };
    sqlx::query_as::<_, SearchItemRow>(
        "SELECT i.id, i.title, i.description, i.list_id, l.name AS list_name, \
                i.updated_at, i.completed \
         FROM items_fts \
         JOIN items i ON i.rowid = items_fts.rowid \
         JOIN lists l ON l.id = i.list_id \
         WHERE l.user_id = ? AND l.archived = 0 AND items_fts MATCH ? \
         ORDER BY bm25(items_fts) \
         LIMIT 50",
    )
    .bind(user_id)
    .bind(fts_query)
    .fetch_all(pool)
    .await
    .map_err(DbError::Sqlx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::{InsertItemInput, insert};
    use crate::test_helpers::{create_test_user, test_pool};

    async fn create_list_for_user(pool: &SqlitePool, user_id: &str, name: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let mut conn = pool.acquire().await.unwrap();
        crate::lists::insert(
            &mut conn,
            &id,
            user_id,
            0,
            name,
            None,
            None,
            "checklist",
            None,
            None,
        )
        .await
        .unwrap();
        id
    }

    #[tokio::test]
    async fn search_items_returns_match() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_list_for_user(&pool, &uid, "Test List").await;
        let item_id = uuid::Uuid::new_v4().to_string();
        insert(
            &pool,
            &InsertItemInput {
                id: item_id,
                list_id: list_id.clone(),
                position: 0,
                title: "uniquefindableterm".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let results = search_items(&pool, &uid, "uniquefindableterm")
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "uniquefindableterm");
        assert_eq!(results[0].list_name, "Test List");
    }

    #[tokio::test]
    async fn search_items_no_results_for_other_user() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let uid2 = create_test_user(&pool).await;
        let list_id = create_list_for_user(&pool, &uid, "L").await;
        let item_id = uuid::Uuid::new_v4().to_string();
        insert(
            &pool,
            &InsertItemInput {
                id: item_id,
                list_id,
                position: 0,
                title: "secretterm".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let results = search_items(&pool, &uid2, "secretterm").await.unwrap();
        assert!(results.is_empty());
    }
}
