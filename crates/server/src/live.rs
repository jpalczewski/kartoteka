//! Live updates: the server-side [`ChangeHub`] and the `GET /api/events` SSE stream.
//!
//! Issue #357. The `change_log` triggers (#356) record every write to user-visible
//! data as a row; [`ChangeHub`] turns new rows into one [`UserChange`] per user and
//! the handler below streams them to that user's browsers over server-sent events.
//! A reconnecting browser gets the changes it missed through `Last-Event-ID`, or a
//! `resync` event when the log no longer reaches back far enough.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::State;
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use futures_util::StreamExt as _;
use futures_util::stream;
use kartoteka_db::changes::{self, ChangeRow};
use kartoteka_db::{ChangeSignal, SqlitePool};
use kartoteka_shared::ChangeTopic;
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::{AppState, extractors::UserId};

/// One user's changes from a single batch of `change_log` rows.
#[derive(Debug, Clone)]
pub struct UserChange {
    /// Owner of the changed data.
    pub user_id: String,
    /// Greatest `change_log.id` covered by this event; becomes the SSE `id`.
    pub last_id: i64,
    /// Changed topics, deduped, in the order they first appeared.
    pub topics: Vec<ChangeTopic>,
}

/// Fan-out of `change_log` rows to per-user subscribers, plus the pump task that
/// reads them (see [`ChangeHub::start`]) and the per-user stream accounting.
#[derive(Clone)]
pub struct ChangeHub {
    tx: broadcast::Sender<UserChange>,
    pool: SqlitePool,
    streams: Arc<Mutex<HashMap<String, usize>>>,
}

/// Broadcast capacity. A subscriber that falls behind gets a `resync`, so this only
/// bounds memory.
const BROADCAST_CAPACITY: usize = 1024;
/// Concurrent SSE streams per user; the next `GET /api/events` gets a 429.
pub const MAX_STREAMS_PER_USER: usize = 8;
/// Rows read per query while draining the log (also the replay limit).
const BATCH_LIMIT: i64 = 1000;
/// Wake the pump anyway if no commit signal arrives (safety net, e.g. lost wake-up).
const FALLBACK_TICK: Duration = Duration::from_secs(2);
/// SQLite fires the commit hook *before* the commit is visible to other
/// connections: wait a moment before reading, which also coalesces bursts.
const COALESCE: Duration = Duration::from_millis(50);
/// How often the pump prunes the log.
const PRUNE_EVERY: Duration = Duration::from_secs(60 * 60);
/// Rows older than this are pruned: a longer reconnect gets a `resync`.
const PRUNE_AFTER: chrono::Duration = chrono::Duration::hours(24);
/// SSE keep-alive comment interval.
const KEEP_ALIVE: Duration = Duration::from_secs(15);
/// Tells nginx (and friends) not to buffer the stream.
const X_ACCEL_BUFFERING: HeaderName = HeaderName::from_static("x-accel-buffering");
/// `http` has no constant for the SSE reconnect header.
const LAST_EVENT_ID: HeaderName = HeaderName::from_static("last-event-id");

impl ChangeHub {
    /// Spawns the pump task. Call once at startup.
    ///
    /// The pump starts at the current log head, so history is never replayed on
    /// startup, and wakes on `signal` or every [`FALLBACK_TICK`].
    pub fn start(pool: SqlitePool, signal: ChangeSignal) -> Self {
        let (tx, _rx) = broadcast::channel(BROADCAST_CAPACITY);
        let hub = Self {
            tx: tx.clone(),
            pool: pool.clone(),
            streams: Arc::new(Mutex::new(HashMap::new())),
        };
        tokio::spawn(pump(pool, signal, tx));
        hub
    }

    /// Subscribes to live changes for every user; the caller filters by `user_id`.
    pub fn subscribe(&self) -> broadcast::Receiver<UserChange> {
        self.tx.subscribe()
    }

    /// The pool this hub pumps. Streams read their replay from it, so a replay and
    /// the pump see the same pool (and therefore the same `change_log`).
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Reserves a stream slot for `user_id`, or `None` when the user already has
    /// [`MAX_STREAMS_PER_USER`] open. The slot is released when the guard drops,
    /// i.e. when the response body ends.
    fn open_stream(&self, user_id: &str) -> Option<StreamGuard> {
        let mut counts = lock(&self.streams);
        let count = counts.entry(user_id.to_string()).or_insert(0);
        if *count >= MAX_STREAMS_PER_USER {
            return None;
        }
        *count += 1;
        Some(StreamGuard {
            user_id: user_id.to_string(),
            streams: Arc::clone(&self.streams),
        })
    }
}

/// Releases one of a user's stream slots on drop.
struct StreamGuard {
    user_id: String,
    streams: Arc<Mutex<HashMap<String, usize>>>,
}

impl Drop for StreamGuard {
    fn drop(&mut self) {
        let mut counts = lock(&self.streams);
        if let Some(count) = counts.get_mut(&self.user_id) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                counts.remove(&self.user_id);
            }
        }
    }
}

/// A poisoned counter lock still holds valid data: the map is never left half-updated.
fn lock(
    counts: &Mutex<HashMap<String, usize>>,
) -> std::sync::MutexGuard<'_, HashMap<String, usize>> {
    counts
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Reads new rows and broadcasts them, forever.
async fn pump(pool: SqlitePool, signal: ChangeSignal, tx: broadcast::Sender<UserChange>) {
    let mut last = match changes::max_id(&pool).await {
        Ok(id) => id,
        Err(e) => {
            warn!(error = %e, "live updates: could not read the change log head");
            0
        }
    };
    let mut next_prune = tokio::time::Instant::now() + PRUNE_EVERY;

    loop {
        tokio::select! {
            () = signal.0.notified() => {}
            () = tokio::time::sleep(FALLBACK_TICK) => {}
        }
        tokio::time::sleep(COALESCE).await;
        drain(&pool, &tx, &mut last).await;

        if tokio::time::Instant::now() >= next_prune {
            match changes::prune(&pool, PRUNE_AFTER).await {
                Ok(0) => {}
                Ok(rows) => info!(rows, "live updates: pruned the change log"),
                Err(e) => warn!(error = %e, "live updates: pruning the change log failed"),
            }
            next_prune = tokio::time::Instant::now() + PRUNE_EVERY;
        }
    }
}

/// Reads every row after `last`, broadcasting one [`UserChange`] per user per batch.
async fn drain(pool: &SqlitePool, tx: &broadcast::Sender<UserChange>, last: &mut i64) {
    loop {
        let rows = match changes::fetch_after(pool, *last, BATCH_LIMIT).await {
            Ok(rows) => rows,
            Err(e) => {
                // A read error must not kill the pump: the next tick retries.
                warn!(error = %e, "live updates: reading the change log failed");
                return;
            }
        };
        if rows.is_empty() {
            return;
        }
        if let Some(max) = rows.last().map(|row| row.id) {
            *last = max;
        }
        for change in group_changes(rows) {
            // No subscribers is not an error.
            let _ = tx.send(change);
        }
    }
}

/// Groups one batch of rows per user, deduping topics and keeping the greatest id.
///
/// Users appear in the order of their first row, so the caller can rely on the
/// broadcast order. Rows with an unknown topic are logged and skipped; the batch's
/// `last_id` (used to advance the log cursor) still covers them.
pub fn group_changes(rows: Vec<ChangeRow>) -> Vec<UserChange> {
    let mut order: Vec<String> = Vec::new();
    let mut by_user: HashMap<String, UserChange> = HashMap::new();

    for row in rows {
        let Some(topic) = parse_topic(&row.topic) else {
            warn!(topic = %row.topic, id = row.id, "live updates: unknown change topic");
            continue;
        };
        let change = by_user.entry(row.user_id.clone()).or_insert_with(|| {
            order.push(row.user_id.clone());
            UserChange {
                user_id: row.user_id.clone(),
                last_id: row.id,
                topics: Vec::new(),
            }
        });
        change.last_id = change.last_id.max(row.id);
        if !change.topics.contains(&topic) {
            change.topics.push(topic);
        }
    }

    order
        .into_iter()
        .filter_map(|user_id| by_user.remove(&user_id))
        .collect()
}

/// Parses a `change_log.topic` string into a [`ChangeTopic`] (serde snake_case names).
fn parse_topic(raw: &str) -> Option<ChangeTopic> {
    serde_json::from_value(serde_json::Value::from(raw)).ok()
}

/// `GET /api/events` — server-sent events with this user's changes.
///
/// Nesting under `/api` runs `require_auth` first, so an unauthenticated request
/// gets its usual 401. The first event is `ready` (or `change`/`resync` after a
/// reconnect), then one `change` per batch of this user's changes.
#[tracing::instrument(skip_all, fields(action = "stream_events"))]
pub async fn events(State(state): State<AppState>, user: UserId, headers: HeaderMap) -> Response {
    let Some(guard) = state.change_hub.open_stream(&user.0) else {
        warn!(
            user_id = %user.0,
            limit = MAX_STREAMS_PER_USER,
            "live updates: too many concurrent event streams"
        );
        return (
            StatusCode::TOO_MANY_REQUESTS,
            "too many concurrent event streams",
        )
            .into_response();
    };

    // Subscribe before reading the replay: a change committed in between is then
    // either in the replay or in the receiver (live events up to the replayed id
    // are dropped below), never lost and never duplicated.
    let receiver = state.change_hub.subscribe();
    let pool = state.change_hub.pool().clone();
    let (initial, seen_through) = initial_events(&pool, &user.0, last_event_id(&headers)).await;

    let live = stream::unfold(
        LiveState {
            receiver,
            pool,
            user_id: user.0,
            seen_through,
            _guard: guard,
        },
        next_live_event,
    );

    let sse = Sse::new(stream::iter(initial).chain(live))
        .keep_alive(KeepAlive::new().interval(KEEP_ALIVE));
    ([(X_ACCEL_BUFFERING, HeaderValue::from_static("no"))], sse).into_response()
}

/// State of the live half of a stream; holds the stream slot guard.
struct LiveState {
    receiver: broadcast::Receiver<UserChange>,
    pool: SqlitePool,
    user_id: String,
    /// Live changes up to this id are already covered by the replay (or dropped).
    seen_through: i64,
    _guard: StreamGuard,
}

/// Next SSE event from the hub: this user's changes, or a `resync` after a lag.
async fn next_live_event(mut state: LiveState) -> Option<(Result<Event, Infallible>, LiveState)> {
    loop {
        match state.receiver.recv().await {
            Ok(change) => {
                if change.user_id != state.user_id || change.last_id <= state.seen_through {
                    continue;
                }
                state.seen_through = change.last_id;
                let event = change_event(change.last_id, &change.topics);
                return Some((Ok(event), state));
            }
            Err(broadcast::error::RecvError::Lagged(missed)) => {
                warn!(
                    missed,
                    "live updates: subscriber lagged behind, sending resync"
                );
                let current = current_max_id(&state.pool).await;
                state.seen_through = current;
                return Some((Ok(resync_event(current)), state));
            }
            // The hub lives as long as the router, so this is shutdown.
            Err(broadcast::error::RecvError::Closed) => return None,
        }
    }
}

/// Events to send before the live stream, and the id live changes must exceed.
///
/// - no `Last-Event-ID`: `ready` with the current log head;
/// - `Last-Event-ID` older than what the log still holds: `resync`;
/// - otherwise the missed topics of this user as one `change` event.
async fn initial_events(
    pool: &SqlitePool,
    user_id: &str,
    last_event_id: Option<i64>,
) -> (Vec<Result<Event, Infallible>>, i64) {
    let Some(after) = last_event_id else {
        let current = current_max_id(pool).await;
        return (vec![Ok(ready_event(current))], current);
    };

    // A pruned window means a replay would silently lose this user's changes.
    let pruned = match changes::min_id(pool).await {
        Ok(Some(min)) => after < min - 1,
        Ok(None) => false,
        Err(e) => {
            warn!(error = %e, "live updates: reading the change log tail failed");
            true
        }
    };
    if pruned {
        let current = current_max_id(pool).await;
        return (vec![Ok(resync_event(current))], current);
    }

    match changes::fetch_for_user_after(pool, user_id, after, BATCH_LIMIT).await {
        // More than one batch was missed: refetch everything rather than replay.
        Ok(rows) if rows.len() as i64 >= BATCH_LIMIT => {
            let current = current_max_id(pool).await;
            (vec![Ok(resync_event(current))], current)
        }
        Ok(rows) => match rows.last() {
            Some(last) => {
                let topics = topics_of(&rows);
                (vec![Ok(change_event(last.id, &topics))], last.id)
            }
            None => (Vec::new(), after),
        },
        Err(e) => {
            warn!(error = %e, user_id, "live updates: replaying changes failed");
            let current = current_max_id(pool).await;
            (vec![Ok(resync_event(current))], current)
        }
    }
}

/// Topics of a batch of rows, deduped, unknown ones skipped (the replay path).
fn topics_of(rows: &[ChangeRow]) -> Vec<ChangeTopic> {
    let mut topics = Vec::new();
    for row in rows {
        let Some(topic) = parse_topic(&row.topic) else {
            warn!(topic = %row.topic, id = row.id, "live updates: unknown change topic");
            continue;
        };
        if !topics.contains(&topic) {
            topics.push(topic);
        }
    }
    topics
}

/// `Last-Event-ID` as a `change_log` id; unparsable values count as absent.
fn last_event_id(headers: &HeaderMap) -> Option<i64> {
    headers
        .get(LAST_EVENT_ID)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse().ok())
}

async fn current_max_id(pool: &SqlitePool) -> i64 {
    match changes::max_id(pool).await {
        Ok(id) => id,
        Err(e) => {
            warn!(error = %e, "live updates: reading the change log head failed");
            0
        }
    }
}

fn ready_event(id: i64) -> Event {
    Event::default()
        .event("ready")
        .id(id.to_string())
        .data("{}")
}

fn resync_event(id: i64) -> Event {
    Event::default()
        .event("resync")
        .id(id.to_string())
        .data("{}")
}

fn change_event(last_id: i64, topics: &[ChangeTopic]) -> Event {
    let payload = serde_json::json!({ "topics": topics });
    Event::default()
        .event("change")
        .id(last_id.to_string())
        .data(payload.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_stream_cap_is_released_on_drop() {
        // The pool is never used, but building it needs the runtime.
        let hub = ChangeHub {
            tx: broadcast::channel(1).0,
            pool: SqlitePool::connect_lazy(":memory:").unwrap(),
            streams: Arc::new(Mutex::new(HashMap::new())),
        };

        let guards: Vec<StreamGuard> = (0..MAX_STREAMS_PER_USER)
            .map(|_| hub.open_stream("u1").expect("slot"))
            .collect();
        assert!(hub.open_stream("u1").is_none(), "ninth stream is refused");
        assert!(
            hub.open_stream("u2").is_some(),
            "other users are unaffected"
        );

        drop(guards);
        assert!(hub.open_stream("u1").is_some(), "slots are back after drop");
    }
}
