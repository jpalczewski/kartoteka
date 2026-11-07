pub mod auth_ctx;
pub mod constants;
pub mod date_utils;
pub mod human_date;
pub mod icon_value;
pub mod locale;
pub mod redirect;
pub mod tag_utils;
pub use constants::*;
pub use icon_value::{IconValue, default_container_icon, default_list_icon, parse_icon_value};
pub use locale::Locale;
pub use redirect::safe_return_to;
pub use tag_utils::{
    DEFAULT_TAG_COLOR, FilterMode, TAG_PALETTE, ancestor_path, build_ancestor_map,
    readable_text_color,
};
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
