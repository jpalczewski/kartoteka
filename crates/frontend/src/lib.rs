/// Library target exposing the api module for unit testing on native targets.
/// The full application runs from main.rs (binary target).
pub mod api;
pub mod state;

pub use state::AdminContext;
