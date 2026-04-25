use leptos::prelude::*;

/// App-level refresh bus. Pages that cache user-scoped data via `Resource`
/// include `GlobalRefresh::get()` in their dep tuple; any component that
/// mutates user-scoped state (e.g. timezone) calls `bump()` to invalidate.
#[derive(Clone, Copy)]
pub struct GlobalRefresh(pub RwSignal<u32>);

impl GlobalRefresh {
    pub fn new() -> Self {
        Self(RwSignal::new(0))
    }

    pub fn get(self) -> u32 {
        self.0.get()
    }

    pub fn bump(self) {
        self.0.update(|n| *n += 1);
    }
}

impl Default for GlobalRefresh {
    fn default() -> Self {
        Self::new()
    }
}
