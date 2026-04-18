pub mod auth;
pub mod comments;
pub mod containers;
pub mod home;
pub mod items;
pub mod lists;
pub mod preferences;
pub mod relations;
pub mod rules;
pub mod settings;
pub mod tags;

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("not found: {0}")]
    NotFound(&'static str),
    #[error("validation: {0}")]
    Validation(&'static str),
    #[error("feature required: {0}")]
    FeatureRequired(&'static str),
    #[error("forbidden")]
    Forbidden,
    #[error("{0}")]
    Internal(String),
    #[error(transparent)]
    Db(#[from] kartoteka_db::DbError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{create_test_user, test_pool};

    #[tokio::test]
    async fn test_pool_and_create_user_work() {
        let pool = test_pool().await;
        let user_id = create_test_user(&pool).await;
        assert!(!user_id.is_empty());
        assert_eq!(user_id.len(), 36);

        let row: (String,) = sqlx::query_as("SELECT role FROM users WHERE id = ?")
            .bind(&user_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(row.0, "user");
    }

    #[test]
    fn domain_error_from_db_error() {
        let db_err = kartoteka_db::DbError::NotFound("item");
        let domain_err: DomainError = db_err.into();
        assert!(matches!(domain_err, DomainError::Db(_)));
    }
}
