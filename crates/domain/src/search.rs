use crate::DomainError;
use kartoteka_db as db;
use sqlx::SqlitePool;

// Re-export for convenience
pub use kartoteka_shared::models::search::{SearchEntityResult, SearchEntityType};

#[tracing::instrument(skip(pool))]
pub async fn search(
    pool: &SqlitePool,
    user_id: &str,
    query: &str,
) -> Result<Vec<SearchEntityResult>, DomainError> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    let rows = db::search::search_items(pool, user_id, query).await?;
    Ok(rows
        .into_iter()
        .map(|r| SearchEntityResult {
            entity_type: SearchEntityType::Item,
            id: r.id,
            name: r.title,
            description: r.description,
            updated_at: r.updated_at,
            list_id: Some(r.list_id),
            list_name: Some(r.list_name),
            list_type: None,
            archived: None,
            completed: Some(r.completed),
            container_id: None,
            parent_list_id: None,
            parent_container_id: None,
            status: None,
        })
        .collect())
}
