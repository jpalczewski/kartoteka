//! MCP server — tools, resources, i18n. Consumed by `crates/server`.

pub mod client_ref;
pub mod i18n;
pub mod resources;
pub mod server;
pub mod tools;

pub use i18n::McpI18n;
pub use server::KartotekaServer;

use http::request::Parts;
use kartoteka_shared::auth_ctx::{UserId, UserLocale};

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("unauthorized")]
    Unauthorized,
    #[error(transparent)]
    Domain(#[from] kartoteka_domain::DomainError),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error("bad request: {0}")]
    BadRequest(String),
}

#[allow(dead_code)]
pub(crate) fn extract_user_id(parts: &Parts) -> Result<String, McpError> {
    parts
        .extensions
        .get::<UserId>()
        .map(|u| u.0.clone())
        .ok_or(McpError::Unauthorized)
}

#[allow(dead_code)]
pub(crate) fn extract_locale(parts: &Parts) -> String {
    parts
        .extensions
        .get::<UserLocale>()
        .map(|l| l.0.clone())
        .unwrap_or_else(|| "en".to_string())
}
