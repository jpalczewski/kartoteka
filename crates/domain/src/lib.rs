pub mod lists;
pub mod rules;

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
