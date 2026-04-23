use crate::McpI18n;
use sqlx::SqlitePool;
use std::sync::Arc;

#[allow(dead_code)]
pub struct KartotekaServer {
    pub(crate) pool: SqlitePool,
    pub(crate) i18n: Arc<McpI18n>,
}

impl KartotekaServer {
    pub fn new(pool: SqlitePool, i18n: Arc<McpI18n>) -> Self {
        Self { pool, i18n }
    }
}
