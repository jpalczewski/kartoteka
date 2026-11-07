//! Shared frontend state.
//!
//! Pure modules (`dnd`, `reorder`) hold no Leptos and no web-sys types, so they
//! are natively testable. `invalidation` is the app-wide cache invalidation
//! bus and only uses Leptos signals.

pub mod dnd;
pub mod invalidation;
pub mod reorder;
