pub mod auth_ctx;
pub mod constants;
pub mod date_utils;
pub mod human_date;
pub mod locale;
pub mod redirect;
pub mod tag_utils;
pub use constants::*;
pub use locale::Locale;
pub use redirect::safe_return_to;
pub use tag_utils::FilterMode;
#[allow(dead_code)]
pub(crate) mod deserializers;
pub mod dto;
pub use dto::*;
pub mod models;
pub use models::*;
pub mod types;
// Canonical domain types live in `types` (they mirror the server's domain types and are what the
// frontend deserializes). Re-exported here so the old crate-root names (`kartoteka_shared::List`,
// …) keep resolving.
pub use types::{
    Container, CreateContainerRequest, CreateListRequest, DateItem, HomeData, Item, ItemChange,
    ItemTagLink, List, ListFeature, ListTagLink, MoveContainerRequest, RecurrenceMode,
    RecurrenceParseError, RecurrenceRule, RecurrenceUnit, Tag, UpdateContainerRequest, UserSetting,
};
pub(crate) mod validation;

#[cfg(test)]
mod tests;
