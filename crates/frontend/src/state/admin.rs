use leptos::prelude::*;

/// App-wide admin state. Provided at app root, updated after session check.
#[derive(Clone, Copy)]
pub struct AdminContext {
    pub is_admin: RwSignal<bool>,
}

impl AdminContext {
    pub fn new() -> Self {
        Self {
            is_admin: RwSignal::new(false),
        }
    }
}

impl Default for AdminContext {
    fn default() -> Self {
        Self::new()
    }
}
