use crate::DbError;
use sqlx::SqlitePool;

#[tracing::instrument(skip(pool))]
pub async fn get_timezone(pool: &SqlitePool, user_id: &str) -> Result<String, DbError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM user_settings WHERE user_id = ? AND key = 'timezone'")
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(DbError::Sqlx)?;

    Ok(row.map(|(v,)| v).unwrap_or_else(|| "UTC".to_string()))
}
