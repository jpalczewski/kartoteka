//! Calendar feed: token management, reminder defaults and iCalendar rendering.
//!
//! [`render`] is pure — no database access, deterministic output. [`tokens`]
//! owns the persistence side: subscribe tokens (only their SHA-256 hash is
//! stored), the user's reminder defaults, and assembling a feed for a token.

mod render;
mod tokens;

pub use render::{FeedItem, render_feed};
pub use tokens::{
    CalendarToken, CalendarTokenCreated, CalendarTokenRow, Feed, create_token, feed_for_token,
    get_reminder_defaults, list_tokens, revoke_token, set_reminder_defaults,
};

#[cfg(test)]
mod tests;
