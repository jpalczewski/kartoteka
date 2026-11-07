//! Live updates: a browser `EventSource` on `/api/events` that feeds the
//! [invalidation](crate::state::invalidation) bus.
//!
//! Data can change without this tab doing anything: MCP tools, another tab or
//! another device. The server announces those changes as server-sent events on
//! `GET /api/events` (same origin, so the session cookie authorises it):
//!
//! - `ready` — the stream is open. It only carries the current event id.
//! - `change` — `{"topics": ["items", "tags"]}`. The topics are collected and
//!   applied to the invalidation bus after a short debounce, so a burst of
//!   events costs one refetch per page.
//! - `resync` — the client may have missed events (replay gap, lagging hub), so
//!   everything is invalidated at once.
//!
//! Only the browser parts are gated behind `hydrate`. The payload parsing, the
//! debounce accumulator and the reconnect backoff are plain functions so they
//! are unit-tested natively.

use std::time::Duration;

use kartoteka_shared::ChangeTopic;
use serde::Deserialize;

/// Reconnect delays, in seconds, after the browser gave up on the stream.
///
/// The browser retries a dropped stream by itself, but a non-200 response (401
/// when logged out, 429, 5xx) makes it fail the connection for good. The last
/// value is the cap.
const BACKOFF_SECONDS: [u64; 4] = [5, 10, 30, 60];

/// Wait before reconnect attempt number `attempt` (zero-based): 5 s, 10 s, 30 s,
/// then 60 s for every further attempt.
pub fn backoff_delay(attempt: u32) -> Duration {
    let index = (attempt as usize).min(BACKOFF_SECONDS.len() - 1);
    Duration::from_secs(BACKOFF_SECONDS[index])
}

/// Topics of a `change` event payload: `{"topics": ["items", "tags"]}`.
///
/// Malformed JSON and unknown topic names yield no topic instead of failing the
/// whole message: the names are decoded one by one, so a topic that a newer
/// server adds does not break an older page.
pub fn parse_topics(data: &str) -> Vec<ChangeTopic> {
    #[derive(Deserialize)]
    struct Payload {
        #[serde(default)]
        topics: Vec<String>,
    }

    let Ok(payload) = serde_json::from_str::<Payload>(data) else {
        return Vec::new();
    };

    let mut pending = PendingTopics::new();
    for name in payload.topics {
        if let Ok(topic) = serde_json::from_value::<ChangeTopic>(serde_json::Value::String(name)) {
            pending.insert(topic);
        }
    }
    pending.take()
}

/// Topics collected between two debounce flushes.
///
/// A set that keeps insertion order, so the invalidation call is deterministic
/// and no topic is invalidated twice.
#[derive(Debug, Default)]
pub struct PendingTopics {
    topics: Vec<ChangeTopic>,
}

impl PendingTopics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a topic, ignoring one that is already pending.
    pub fn insert(&mut self, topic: ChangeTopic) {
        if !self.topics.contains(&topic) {
            self.topics.push(topic);
        }
    }

    pub fn extend(&mut self, topics: impl IntoIterator<Item = ChangeTopic>) {
        for topic in topics {
            self.insert(topic);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.topics.is_empty()
    }

    pub fn len(&self) -> usize {
        self.topics.len()
    }

    /// Take the pending topics, leaving the accumulator empty.
    pub fn take(&mut self) -> Vec<ChangeTopic> {
        std::mem::take(&mut self.topics)
    }
}

/// Start listening for live updates.
///
/// Call once in `App` for an authenticated session, right after
/// [`provide_invalidation`](crate::state::invalidation::provide_invalidation).
/// Nothing happens on the server (SSR) or in a browser without `EventSource`.
#[cfg(feature = "hydrate")]
pub fn provide_live_updates() {
    browser::provide();
}

/// Start listening for live updates.
///
/// No-op: this build has no browser.
#[cfg(not(feature = "hydrate"))]
pub fn provide_live_updates() {}

#[cfg(feature = "hydrate")]
mod browser {
    use std::cell::{Cell, RefCell};
    use std::rc::{Rc, Weak};
    use std::time::Duration;

    use leptos::prelude::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;
    use web_sys::{EventSource, MessageEvent, VisibilityState};

    use crate::state::invalidation::Invalidation;

    use super::{PendingTopics, backoff_delay, parse_topics};

    /// Same origin, so the browser sends the session cookie.
    const EVENTS_URL: &str = "/api/events";

    /// How long to wait for further `change` events before invalidating.
    const DEBOUNCE: Duration = Duration::from_millis(300);

    /// A DOM event handler. Every listener here ignores the arguments it does
    /// not need, so one closure type covers all of them.
    type Handler = Closure<dyn FnMut(web_sys::Event)>;

    /// One connection attempt: the `EventSource` and the closures attached to
    /// it. Dropping it detaches the listeners; the source is closed first.
    struct Stream {
        source: EventSource,
        /// Kept only so the handlers stay alive while the stream is open.
        _handlers: Vec<Handler>,
    }

    impl Stream {
        fn is_closed(&self) -> bool {
            self.source.ready_state() == EventSource::CLOSED
        }
    }

    /// State shared by all callbacks of one subscription.
    ///
    /// Held in a [`StoredValue`] (reactive arena) and behind an `Rc` for the
    /// callbacks; they keep only a [`Weak`] reference, so nothing leaks if a
    /// cleanup is missed.
    struct Live {
        invalidation: Invalidation,
        /// Failed reconnect attempts since the last successful `open`.
        attempt: Cell<u32>,
        stream: RefCell<Option<Stream>>,
        visibility: RefCell<Option<Handler>>,
        pending: RefCell<PendingTopics>,
        debounce: Cell<Option<TimeoutHandle>>,
        retry: Cell<Option<TimeoutHandle>>,
        /// Set by the cleanup: no timer may reopen the stream any more.
        stopped: Cell<bool>,
    }

    impl Live {
        fn new(invalidation: Invalidation) -> Rc<Self> {
            Rc::new(Self {
                invalidation,
                attempt: Cell::new(0),
                stream: RefCell::new(None),
                visibility: RefCell::new(None),
                pending: RefCell::new(PendingTopics::new()),
                debounce: Cell::new(None),
                retry: Cell::new(None),
                stopped: Cell::new(false),
            })
        }

        /// Watch the tab's visibility and open the stream.
        fn start(self: &Rc<Self>) {
            self.watch_visibility();
            self.open();
        }

        /// Connect (or reconnect) with a fresh `EventSource`.
        ///
        /// Handlers only ever ask for a delayed reopen, never for an immediate
        /// one, so the previous stream is dropped outside of its own callbacks.
        fn open(self: &Rc<Self>) {
            self.cancel_retry();
            if self.stopped.get() {
                return;
            }
            self.stream.borrow_mut().take();

            let Ok(source) = EventSource::new(EVENTS_URL) else {
                // No `EventSource` in this browser: nothing to open, nowhere to retry.
                return;
            };

            let live = Rc::downgrade(self);

            let on_open = {
                let live = Weak::clone(&live);
                Closure::<dyn FnMut(web_sys::Event)>::new(move |_event: web_sys::Event| {
                    if let Some(live) = live.upgrade() {
                        // A healthy stream resets the backoff.
                        live.attempt.set(0);
                    }
                })
            };
            source.set_onopen(Some(on_open.as_ref().unchecked_ref()));

            let on_error = {
                let live = Weak::clone(&live);
                let source = source.clone();
                Closure::<dyn FnMut(web_sys::Event)>::new(move |_event: web_sys::Event| {
                    // The browser retries a dropped stream by itself (and sends
                    // `Last-Event-ID`). After a non-200 response it fails the
                    // connection instead: `readyState` is `CLOSED` and it never
                    // retries, so we have to reopen it ourselves.
                    if source.ready_state() != EventSource::CLOSED {
                        return;
                    }
                    if let Some(live) = live.upgrade() {
                        source.close();
                        live.schedule_retry();
                    }
                })
            };
            source.set_onerror(Some(on_error.as_ref().unchecked_ref()));

            let on_change = {
                let live = Weak::clone(&live);
                Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
                    let Some(live) = live.upgrade() else {
                        return;
                    };
                    let Ok(message) = event.dyn_into::<MessageEvent>() else {
                        return;
                    };
                    let data = message.data().as_string().unwrap_or_default();
                    let topics = parse_topics(&data);
                    if topics.is_empty() {
                        return;
                    }
                    live.pending.borrow_mut().extend(topics);
                    live.schedule_flush();
                })
            };
            listen(&source, "change", &on_change);

            let on_resync = {
                let live = Weak::clone(&live);
                Closure::<dyn FnMut(web_sys::Event)>::new(move |_event: web_sys::Event| {
                    let Some(live) = live.upgrade() else {
                        return;
                    };
                    // The server could not replay what we missed, so everything
                    // may be stale and the pending topics add nothing to that.
                    live.cancel_flush();
                    live.pending.borrow_mut().take();
                    live.invalidation.invalidate_all();
                })
            };
            listen(&source, "resync", &on_resync);

            *self.stream.borrow_mut() = Some(Stream {
                source,
                _handlers: vec![on_open, on_error, on_change, on_resync],
            });
        }

        /// Reopen after the browser gave up, backing off on every attempt.
        fn schedule_retry(self: &Rc<Self>) {
            if self.stopped.get() || self.retry.get().is_some() {
                return;
            }
            let delay = backoff_delay(self.attempt.get());
            self.attempt.set(self.attempt.get().saturating_add(1));

            let live = Rc::downgrade(self);
            let handle = set_timeout_with_handle(
                move || {
                    if let Some(live) = live.upgrade() {
                        live.open();
                    }
                },
                delay,
            );
            self.retry.set(handle.ok());
        }

        fn cancel_retry(&self) {
            if let Some(handle) = self.retry.take() {
                handle.clear();
            }
        }

        /// Coalesce a burst of `change` events into one invalidation.
        fn schedule_flush(self: &Rc<Self>) {
            self.cancel_flush();

            let live = Rc::downgrade(self);
            let handle = set_timeout_with_handle(
                move || {
                    if let Some(live) = live.upgrade() {
                        live.flush();
                    }
                },
                DEBOUNCE,
            );
            self.debounce.set(handle.ok());
        }

        fn cancel_flush(&self) {
            if let Some(handle) = self.debounce.take() {
                handle.clear();
            }
        }

        fn flush(&self) {
            self.debounce.set(None);
            let topics = self.pending.borrow_mut().take();
            if !topics.is_empty() {
                self.invalidation.invalidate_many(&topics);
            }
        }

        /// Reopen the tab: a stream that was closed while the tab was hidden
        /// missed events, so everything is refetched.
        fn check_visibility(self: &Rc<Self>) {
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };
            if document.visibility_state() != VisibilityState::Visible {
                return;
            }

            let closed = self.stream.borrow().as_ref().is_none_or(Stream::is_closed);
            if !closed {
                return;
            }

            self.cancel_retry();
            self.invalidation.invalidate_all();
            self.open();
        }

        fn watch_visibility(self: &Rc<Self>) {
            if self.visibility.borrow().is_some() {
                return;
            }
            let Some(document) = web_sys::window().and_then(|window| window.document()) else {
                return;
            };

            let live = Rc::downgrade(self);
            let handler =
                Closure::<dyn FnMut(web_sys::Event)>::new(move |_event: web_sys::Event| {
                    if let Some(live) = live.upgrade() {
                        live.check_visibility();
                    }
                });
            listen(&document, "visibilitychange", &handler);
            *self.visibility.borrow_mut() = Some(handler);
        }

        /// Close the stream, cancel the timers and drop every closure.
        fn stop(&self) {
            self.stopped.set(true);
            self.cancel_retry();
            self.cancel_flush();

            if let Some(handler) = self.visibility.borrow_mut().take() {
                if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                    let function: &js_sys::Function = handler.as_ref().unchecked_ref();
                    _ = document.remove_event_listener_with_callback("visibilitychange", function);
                }
            }
            if let Some(stream) = self.stream.borrow_mut().take() {
                stream.source.close();
            }
        }
    }

    /// Register a named listener, ignoring a failure to attach it: without it
    /// the stream stays open but this one event kind is not handled.
    fn listen(target: &web_sys::EventTarget, event: &str, handler: &Handler) {
        let function: &js_sys::Function = handler.as_ref().unchecked_ref();
        _ = target.add_event_listener_with_callback(event, function);
    }

    /// Start live updates for this browser session.
    pub fn provide() {
        let live: StoredValue<Rc<Live>, LocalStorage> =
            StoredValue::new_local(Live::new(crate::state::invalidation::use_invalidation()));

        // The effect never runs during SSR, and it has no reactive dependency,
        // so this opens one connection for the lifetime of the app.
        Effect::new(move |_| {
            if let Some(live) = live.try_get_value() {
                live.start();
            }
        });

        on_cleanup(move || {
            if let Some(live) = live.try_get_value() {
                live.stop();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_topics_of_a_change_event() {
        assert_eq!(
            parse_topics(r#"{"topics":["items","tags"]}"#),
            vec![ChangeTopic::Items, ChangeTopic::Tags]
        );
        assert_eq!(
            parse_topics(r#"{"topics":["time_entries"]}"#),
            vec![ChangeTopic::TimeEntries]
        );
        assert_eq!(parse_topics(r#"{"topics":[]}"#), Vec::new());
    }

    #[test]
    fn ignores_malformed_payloads() {
        assert_eq!(parse_topics(""), Vec::new());
        assert_eq!(parse_topics("not json"), Vec::new());
        assert_eq!(parse_topics("{}"), Vec::new());
        assert_eq!(parse_topics(r#"{"topics":"items"}"#), Vec::new());
    }

    #[test]
    fn ignores_unknown_topics_but_keeps_the_known_ones() {
        assert_eq!(
            parse_topics(r#"{"topics":["items","not_a_topic","tags"]}"#),
            vec![ChangeTopic::Items, ChangeTopic::Tags]
        );
    }

    #[test]
    fn dedupes_topics_within_one_payload() {
        assert_eq!(
            parse_topics(r#"{"topics":["items","items"]}"#),
            vec![ChangeTopic::Items]
        );
    }

    #[test]
    fn pending_topics_merge_and_are_taken_once() {
        let mut pending = PendingTopics::new();
        assert!(pending.is_empty());

        pending.insert(ChangeTopic::Lists);
        pending.extend([ChangeTopic::Items, ChangeTopic::Lists]);
        assert_eq!(pending.len(), 2);

        assert_eq!(pending.take(), vec![ChangeTopic::Lists, ChangeTopic::Items]);
        assert!(pending.is_empty());

        // Topics seen after the flush are collected again.
        pending.insert(ChangeTopic::Tags);
        assert_eq!(pending.take(), vec![ChangeTopic::Tags]);
    }

    #[test]
    fn backoff_starts_at_five_seconds_and_caps_at_sixty() {
        assert_eq!(backoff_delay(0), Duration::from_secs(5));
        assert_eq!(backoff_delay(1), Duration::from_secs(10));
        assert_eq!(backoff_delay(2), Duration::from_secs(30));
        assert_eq!(backoff_delay(3), Duration::from_secs(60));
        assert_eq!(backoff_delay(4), Duration::from_secs(60));
        assert_eq!(backoff_delay(u32::MAX), Duration::from_secs(60));
    }
}
