//! The signed-in user's "today" and display locale, for item views.
//!
//! "Today" comes from the server in the user's timezone (`get_user_today`), never from the
//! browser clock: the server render and the hydrated page must agree on which items are
//! due today or overdue. It refetches when settings change (timezone) and on the full
//! resync the live-updates client does when a tab becomes visible again, which also covers
//! a page left open past midnight.
//!
//! Read [`use_today`] inside a `<Suspense>`/`<Transition>` (item rows always are): the
//! resource is serialized for hydration only when it is read inside one.

use chrono::NaiveDate;
use kartoteka_shared::{ChangeTopic, Locale};
use leptos::prelude::*;

use crate::server_fns::settings::get_user_today;
use crate::state::invalidation::use_invalidation;

/// App-wide resource with the user's today, provided by [`provide_user_today`].
#[derive(Clone, Copy)]
pub struct UserToday(Resource<Result<String, ServerFnError>>);

/// Provide [`UserToday`]. Call once in `App`, after the invalidation bus is provided.
pub fn provide_user_today() {
    let invalidation = use_invalidation();
    let today = Resource::new(
        move || invalidation.track(&[ChangeTopic::Settings]),
        |_| get_user_today(),
    );
    provide_context(UserToday(today));
}

/// The user's today; `None` while loading, signed out, or on error.
pub fn use_today() -> Signal<Option<NaiveDate>> {
    let UserToday(today) = expect_context::<UserToday>();
    Signal::derive(move || {
        today
            .get()
            .and_then(Result::ok)
            .and_then(|date| date.parse::<NaiveDate>().ok())
    })
}

/// The UI locale for formatting (dates, quantities). Unknown languages fall back to English.
pub fn use_locale() -> Signal<Locale> {
    let i18n = expect_context::<leptos_fluent::I18n>();
    Signal::derive(move || {
        i18n.language
            .get()
            .id
            .language
            .as_str()
            .parse()
            .unwrap_or_default()
    })
}
