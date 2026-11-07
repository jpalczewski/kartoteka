//! Shared frontend state.
//!
//! Pure modules (`dnd`, `reorder`) hold no Leptos and no web-sys types, so they
//! are natively testable. `invalidation` is the app-wide cache invalidation
//! bus and only uses Leptos signals. `live_updates` keeps a server-sent-events
//! connection to the API and feeds that bus.

pub mod dnd;
pub mod invalidation;
pub mod item_actions;
pub mod item_store;
pub mod live_updates;
pub mod reorder;
