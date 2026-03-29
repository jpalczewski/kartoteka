pub mod constants;
pub(crate) mod deserializers;
pub mod dto;
pub mod models;

// Flat re-exports for backward compatibility (no churn in crates/api imports)
pub use constants::*;
pub use dto::*;
pub use models::*;

// Re-export deserializer functions at crate root for test compatibility
#[cfg(test)]
pub(crate) use deserializers::bool_from_number;
#[cfg(test)]
pub(crate) use deserializers::features_from_json;
#[cfg(test)]
pub(crate) use deserializers::u32_from_number;

#[cfg(test)]
mod tests;
