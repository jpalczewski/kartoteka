pub mod auth;
pub mod calendar;
pub mod comments;
pub mod containers;
pub mod home;
pub mod items;
pub mod lists;
pub mod locations;
pub mod paging;
pub mod preferences;
pub mod relations;
pub mod rules;
pub mod search;
pub mod settings;
pub mod tags;
pub mod templates;
pub mod time_entries;

use std::collections::HashSet;

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("not found: {0}")]
    NotFound(&'static str),
    #[error("validation: {0}")]
    Validation(&'static str),
    #[error("feature required: {0}")]
    FeatureRequired(&'static str),
    #[error("already exists: {kind} id={id}")]
    AlreadyExists { kind: &'static str, id: String },
    #[error("forbidden")]
    Forbidden,
    #[error("{0}")]
    Internal(String),
    #[error(transparent)]
    Db(#[from] kartoteka_db::DbError),
}

impl DomainError {
    pub fn ensure_unique(found: Option<String>, kind: &'static str) -> Result<(), Self> {
        match found {
            Some(id) => Err(Self::AlreadyExists { kind, id }),
            None => Ok(()),
        }
    }

    /// Validate a caller-supplied id list against the ids that actually live in
    /// the target scope (`scope_ids`, already ownership-filtered).
    ///
    /// Returns `NotFound(kind)` for any id outside the scope (foreign or
    /// cross-scope id) and `Validation("duplicate_id")` for repeats, so the
    /// caller can abort a reorder before writing anything.
    /// # Errors
    /// See above.
    pub fn ensure_ids_in_scope(
        scope_ids: &[String],
        requested: &[String],
        kind: &'static str,
    ) -> Result<(), Self> {
        let known: HashSet<&str> = scope_ids.iter().map(String::as_str).collect();
        let mut seen = HashSet::with_capacity(requested.len());
        for id in requested {
            if !known.contains(id.as_str()) {
                return Err(Self::NotFound(kind));
            }
            if !seen.insert(id.as_str()) {
                return Err(Self::Validation("duplicate_id"));
            }
        }
        Ok(())
    }
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
