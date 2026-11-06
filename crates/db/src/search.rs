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

/// Minimum number of characters a token needs before it is matched as a prefix.
/// Shorter tokens are matched exactly, so a query like `PR #257` does not turn
/// `PR` into a wildcard that matches every word starting with `pr`.
const MIN_PREFIX_CHARS: usize = 3;

/// Build a safe FTS5 query from free-form user input.
///
/// Each whitespace-delimited token has any `"` stripped. Tokens with at least
/// [`MIN_PREFIX_CHARS`] characters become `"token"*` so partial matches work;
/// shorter tokens become the exact phrase `"token"`. Emptied tokens are dropped.
/// Character counts use `chars()` rather than bytes, so Polish letters count once.
fn fts5_prefix_query(query: &str) -> Option<String> {
    let terms: Vec<String> = query
        .split_whitespace()
        .map(|t| t.replace('"', ""))
        .filter(|t| !t.is_empty())
        .map(|t| {
            if t.chars().count() >= MIN_PREFIX_CHARS {
                format!("\"{t}\"*")
            } else {
                format!("\"{t}\"")
            }
        })
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
            &crate::lists::InsertListInput {
                id: id.clone(),
                user_id: user_id.to_owned(),
                position: 0,
                name: name.to_owned(),
                list_type: "checklist".to_owned(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        id
    }

    #[test]
    fn fts5_prefix_query_keeps_short_tokens_exact() {
        assert_eq!(
            fts5_prefix_query("PR #257").as_deref(),
            Some("\"PR\" \"#257\"*")
        );
        assert_eq!(fts5_prefix_query("ab").as_deref(), Some("\"ab\""));
        assert_eq!(fts5_prefix_query("zak").as_deref(), Some("\"zak\"*"));
    }

    #[test]
    fn fts5_prefix_query_counts_chars_not_bytes() {
        // "żó" is 2 characters but 4 bytes.
        assert_eq!(fts5_prefix_query("żó").as_deref(), Some("\"żó\""));
    }

    #[test]
    fn fts5_prefix_query_drops_empty_tokens_and_quotes() {
        assert_eq!(fts5_prefix_query("  \"\"  "), None);
        assert_eq!(
            fts5_prefix_query("a\"bc\" de").as_deref(),
            Some("\"abc\"* \"de\"")
        );
    }

    #[tokio::test]
    async fn search_items_matches_short_token_exactly() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list_id = create_list_for_user(&pool, &uid, "PR List").await;
        for title in ["PR 257 sqlite indexes", "Przelew za prąd 257"] {
            insert(
                &pool,
                &InsertItemInput {
                    id: uuid::Uuid::new_v4().to_string(),
                    list_id: list_id.clone(),
                    title: title.to_owned(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        }

        let results = search_items(&pool, &uid, "PR #257").await.unwrap();
        assert_eq!(results.len(), 1, "got {results:#?}");
        assert_eq!(results[0].title, "PR 257 sqlite indexes");

        // Longer tokens still match as prefixes.
        let results = search_items(&pool, &uid, "przel").await.unwrap();
        assert_eq!(results.len(), 1, "got {results:#?}");
        assert_eq!(results[0].title, "Przelew za prąd 257");
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
