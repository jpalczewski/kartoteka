//! MCP tool handlers, one module per entity area.
//!
//! Each module carries the params structs for its tools and a
//! `#[tool_router(router = <area>_router, vis = "pub(crate)")]` block on
//! [`KartotekaServer`](crate::server::KartotekaServer). `KartotekaServer::new`
//! merges them into the single router the server dispatches on, so adding a
//! tool means touching one area file, its params and `annotations.rs`.

pub mod comments;
pub mod containers;
pub mod items;
pub mod lists;
pub mod pending_deletions;
pub mod relations;
pub mod search;
pub mod tags;
pub mod templates;
pub mod time;
