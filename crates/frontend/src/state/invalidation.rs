//! Per-topic cache invalidation and the mutation helpers built on it.
//!
//! User-scoped data is cached by Leptos [`Resource`]s. A `Resource` that loads
//! mutable data reads [`Invalidation::track`] for the topics it depends on in
//! its source (key) tuple, so any later invalidation of one of those topics
//! makes it refetch. Mutations call [`mutate`] / [`mutate_with`], which
//! invalidate the affected topics on success and toast the error otherwise.
//!
//! The counters are plain signals, so everything here works in SSR and in the
//! browser.

use std::fmt::Display;
use std::future::Future;

use kartoteka_shared::ChangeTopic;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::app::{ToastContext, ToastKind};

/// One invalidation counter per [`ChangeTopic`].
///
/// Cheap to copy: all state lives in the reactive arena, so the value can be
/// captured by every closure that needs it.
#[derive(Clone, Copy)]
pub struct Invalidation {
    lists: RwSignal<u32>,
    items: RwSignal<u32>,
    tags: RwSignal<u32>,
    containers: RwSignal<u32>,
    settings: RwSignal<u32>,
    comments: RwSignal<u32>,
    relations: RwSignal<u32>,
    time_entries: RwSignal<u32>,
    templates: RwSignal<u32>,
    locations: RwSignal<u32>,
}

impl Invalidation {
    pub fn new() -> Self {
        Self {
            lists: RwSignal::new(0),
            items: RwSignal::new(0),
            tags: RwSignal::new(0),
            containers: RwSignal::new(0),
            settings: RwSignal::new(0),
            comments: RwSignal::new(0),
            relations: RwSignal::new(0),
            time_entries: RwSignal::new(0),
            templates: RwSignal::new(0),
            locations: RwSignal::new(0),
        }
    }

    fn counter(self, topic: ChangeTopic) -> RwSignal<u32> {
        match topic {
            ChangeTopic::Lists => self.lists,
            ChangeTopic::Items => self.items,
            ChangeTopic::Tags => self.tags,
            ChangeTopic::Containers => self.containers,
            ChangeTopic::Settings => self.settings,
            ChangeTopic::Comments => self.comments,
            ChangeTopic::Relations => self.relations,
            ChangeTopic::TimeEntries => self.time_entries,
            ChangeTopic::Templates => self.templates,
            ChangeTopic::Locations => self.locations,
        }
    }

    /// Reactive token for a set of topics: put it in a `Resource` source tuple
    /// to refetch whenever any of the listed topics is invalidated.
    ///
    /// The value is the sum of the tracked counters. They only ever increase,
    /// so the token changes with every invalidation of any listed topic.
    pub fn track(self, topics: &[ChangeTopic]) -> u64 {
        topics.iter().fold(0u64, |total, topic| {
            total.wrapping_add(u64::from(self.counter(*topic).get()))
        })
    }

    /// Mark one topic's cached data as stale.
    pub fn invalidate(self, topic: ChangeTopic) {
        self.counter(topic)
            .update(|count| *count = count.wrapping_add(1));
    }

    /// Mark several topics as stale.
    pub fn invalidate_many(self, topics: &[ChangeTopic]) {
        for topic in topics {
            self.invalidate(*topic);
        }
    }

    /// Mark every topic as stale (e.g. after a timezone or locale change).
    pub fn invalidate_all(self) {
        self.invalidate_many(&ChangeTopic::ALL);
    }
}

impl Default for Invalidation {
    fn default() -> Self {
        Self::new()
    }
}

/// Provide the app-wide [`Invalidation`] to every component.
pub fn provide_invalidation() {
    provide_context(Invalidation::new());
}

/// Access the app-wide [`Invalidation`].
///
/// Like any `use_context` call this must run while an owner is active — in a
/// component body, an event handler, or a `Callback` created in one.
pub fn use_invalidation() -> Invalidation {
    use_context::<Invalidation>().expect("Invalidation missing")
}

/// Run a mutation in the background: on success invalidate `topics`, on error
/// push the error as an error toast.
///
/// Both the invalidation context and the toast context are resolved before the
/// future is spawned; they are not available inside the spawned task.
pub fn mutate<T, E>(fut: impl Future<Output = Result<T, E>> + 'static, topics: &[ChangeTopic])
where
    T: 'static,
    E: Display + 'static,
{
    mutate_with(fut, topics, |_| {});
}

/// Like [`mutate`], but also runs `on_ok` with the mutation result after the
/// affected topics were invalidated. Use it for custom success flows (resetting
/// a form, closing a confirm dialog, showing a success toast).
pub fn mutate_with<T, E>(
    fut: impl Future<Output = Result<T, E>> + 'static,
    topics: &[ChangeTopic],
    on_ok: impl FnOnce(T) + 'static,
) where
    T: 'static,
    E: Display + 'static,
{
    let invalidation = use_invalidation();
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let topics = topics.to_vec();

    spawn_local(async move {
        match fut.await {
            Ok(value) => {
                invalidation.invalidate_many(&topics);
                on_ok(value);
            }
            Err(e) => toast.push(e.to_string(), ToastKind::Error),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_invalidation(test: impl FnOnce(Invalidation)) {
        Owner::new().with(|| test(Invalidation::new()));
    }

    #[test]
    fn counters_are_tracked_per_topic() {
        with_invalidation(|invalidation| {
            assert_eq!(invalidation.track(&[ChangeTopic::Lists]), 0);

            invalidation.invalidate(ChangeTopic::Lists);

            assert_eq!(invalidation.track(&[ChangeTopic::Lists]), 1);
            assert_eq!(invalidation.track(&[ChangeTopic::Items]), 0);
        });
    }

    #[test]
    fn track_changes_when_any_listed_topic_changes() {
        with_invalidation(|invalidation| {
            let tracked = [ChangeTopic::Items, ChangeTopic::Tags];

            assert_eq!(invalidation.track(&tracked), 0);
            invalidation.invalidate(ChangeTopic::Tags);
            assert_eq!(invalidation.track(&tracked), 1);
            invalidation.invalidate(ChangeTopic::Items);
            assert_eq!(invalidation.track(&tracked), 2);
        });
    }

    #[test]
    fn invalidate_many_bumps_only_the_given_topics() {
        with_invalidation(|invalidation| {
            invalidation.invalidate_many(&[ChangeTopic::Lists, ChangeTopic::Comments]);

            assert_eq!(invalidation.track(&[ChangeTopic::Lists]), 1);
            assert_eq!(invalidation.track(&[ChangeTopic::Comments]), 1);
            assert_eq!(invalidation.track(&[ChangeTopic::Items]), 0);
        });
    }

    #[test]
    fn invalidate_all_bumps_every_topic() {
        with_invalidation(|invalidation| {
            invalidation.invalidate_all();

            for topic in ChangeTopic::ALL {
                assert_eq!(invalidation.track(&[topic]), 1, "{topic:?} was not bumped");
            }
        });
    }

    #[test]
    fn provided_invalidation_is_shared() {
        Owner::new().with(|| {
            provide_invalidation();
            let provided = use_invalidation();
            provided.invalidate(ChangeTopic::Settings);

            let used = use_invalidation();
            assert_eq!(used.track(&[ChangeTopic::Settings]), 1);
        });
    }
}
