//! Calendar feed tokens and feed assembly.
//!
//! A token is a 32-byte random secret shown to the user exactly once; only its
//! lowercase hex SHA-256 hash is persisted. Tokens are either user-wide or
//! scoped to a single list (which implicitly covers that list's direct
//! sublists). A feed is rendered for the token's owner's timezone and reminder
//! defaults, so the calendar client never needs credentials.

use crate::DomainError;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Duration;
use chrono_tz::Tz;
use kartoteka_db as db;
use kartoteka_shared::{ReminderDefaults, constants::REMINDER_DEFAULTS_KEY};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use super::render::{FeedItem, render_feed};

pub use kartoteka_db::calendar_tokens::CalendarTokenRow;

/// A calendar token as exposed by the domain (never carries the token itself).
pub type CalendarToken = CalendarTokenRow;

/// Upper bound on the number of live tokens per user.
const MAX_TOKENS_PER_USER: usize = 20;
/// Upper bound on the token name length, in characters.
const MAX_NAME_CHARS: usize = 100;
/// Entropy of a token in bytes — 43 base64url characters without padding.
const TOKEN_BYTES: usize = 32;
/// Length of an encoded token.
const TOKEN_LEN: usize = 43;
/// How far back the feed looks, so recently past items stay visible.
const FEED_DAYS_BACK: i64 = 30;
/// Length of the hex prefix used in the `ETag`.
const ETAG_HEX_CHARS: usize = 32;
/// Calendar name for tokens that cover every list of the user.
const ALL_LISTS_CALENDAR_NAME: &str = "Kartoteka";
/// Date format accepted by the feed query.
const DATE_FORMAT: &str = "%Y-%m-%d";
/// Characters of the unpadded base64url alphabet.
const BASE64URL_EXTRA: [u8; 2] = *b"-_";
/// Lowercase hex digits.
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Result of token creation — the plaintext `token` is returned once and never stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarTokenCreated {
    pub id: String,
    pub token: String,
    pub name: String,
    pub list_id: Option<String>,
}

/// A rendered calendar feed and its strong `ETag`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feed {
    pub body: String,
    pub etag: String,
}

/// Create a subscribe token for the user, optionally scoped to one of their lists.
///
/// Rules:
/// - Name is trimmed, must be non-empty and at most [`MAX_NAME_CHARS`] characters
/// - A list-scoped token requires an existing list owned by the user
/// - At most [`MAX_TOKENS_PER_USER`] tokens per user
/// - The plaintext token is returned once; only `sha256(token)` is stored
#[tracing::instrument(skip(pool))]
pub async fn create_token(
    pool: &SqlitePool,
    user_id: &str,
    name: &str,
    list_id: Option<&str>,
) -> Result<CalendarTokenCreated, DomainError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DomainError::Validation("calendar_token_name_empty"));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(DomainError::Validation("calendar_token_name_too_long"));
    }

    if let Some(list_id) = list_id
        && db::lists::get_one(pool, list_id, user_id).await?.is_none()
    {
        return Err(DomainError::NotFound("list"));
    }

    let existing = db::calendar_tokens::list_by_user(pool, user_id).await?;
    if existing.len() >= MAX_TOKENS_PER_USER {
        return Err(DomainError::Validation("calendar_token_limit"));
    }

    let token = generate_token()?;
    let id = uuid::Uuid::new_v4().to_string();
    let hash = hash_token(&token);
    let row = db::calendar_tokens::create(pool, &id, user_id, list_id, name, &hash).await?;

    Ok(CalendarTokenCreated {
        id: row.id,
        token,
        name: row.name,
        list_id: row.list_id,
    })
}

/// List the user's tokens (metadata only, never the token or its hash).
#[tracing::instrument(skip(pool))]
pub async fn list_tokens(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<CalendarToken>, DomainError> {
    Ok(db::calendar_tokens::list_by_user(pool, user_id).await?)
}

/// Revoke a token. `NotFound` when the token does not exist or is not owned by the user.
#[tracing::instrument(skip(pool))]
pub async fn revoke_token(
    pool: &SqlitePool,
    user_id: &str,
    token_id: &str,
) -> Result<(), DomainError> {
    if !db::calendar_tokens::delete(pool, token_id, user_id).await? {
        return Err(DomainError::NotFound("calendar_token"));
    }
    Ok(())
}

/// Read the user's reminder defaults. Missing or unparsable JSON yields the default set.
#[tracing::instrument(skip(pool))]
pub async fn get_reminder_defaults(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<ReminderDefaults, DomainError> {
    match db::settings::get(pool, user_id, REMINDER_DEFAULTS_KEY).await? {
        Some(row) => Ok(serde_json::from_str(&row.value).unwrap_or_default()),
        None => Ok(ReminderDefaults::default()),
    }
}

/// Persist the user's reminder defaults after validating them.
#[tracing::instrument(skip(pool, value))]
pub async fn set_reminder_defaults(
    pool: &SqlitePool,
    user_id: &str,
    value: &ReminderDefaults,
) -> Result<(), DomainError> {
    value
        .validate()
        .map_err(|_| DomainError::Validation("invalid_reminder_defaults"))?;
    let json = serde_json::to_string(value)
        .map_err(|e| DomainError::Internal(format!("reminder defaults serialize: {e}")))?;
    db::settings::set(pool, user_id, REMINDER_DEFAULTS_KEY, &json).await?;
    Ok(())
}

/// Render the feed for a subscribe token.
///
/// Returns `Ok(None)` for an unknown, revoked or malformed token, and for a
/// list-scoped token whose list has since been deleted.
#[tracing::instrument(skip(pool, token))]
pub async fn feed_for_token(pool: &SqlitePool, token: &str) -> Result<Option<Feed>, DomainError> {
    // Cheap shape check first: the database only ever stores 43-char hashes of tokens.
    if !is_well_formed(token) {
        return Ok(None);
    }
    let Some(row) = db::calendar_tokens::find_by_hash(pool, &hash_token(token)).await? else {
        return Ok(None);
    };

    let tz = resolve_timezone(pool, &row.user_id).await?;
    let from_date = (chrono::Utc::now().with_timezone(&tz).date_naive()
        - Duration::days(FEED_DAYS_BACK))
    .format(DATE_FORMAT)
    .to_string();
    let reminders = get_reminder_defaults(pool, &row.user_id).await?;

    let calendar_name = match row.list_id.as_deref() {
        Some(list_id) => match db::lists::get_one(pool, list_id, &row.user_id).await? {
            Some(list) => list.name,
            None => return Ok(None),
        },
        None => ALL_LISTS_CALENDAR_NAME.to_string(),
    };

    let rows = db::items::for_calendar_feed(pool, &row.user_id, row.list_id.as_deref(), &from_date)
        .await?;
    let items: Vec<FeedItem> = rows.into_iter().map(FeedItem::from).collect();
    let body = render_feed(&calendar_name, &items, tz, &reminders);
    let etag = format!("\"{}\"", etag_digest(&body));

    // Best effort: a failed bookkeeping write must not break the feed.
    let _ = db::calendar_tokens::touch_last_used(pool, &row.id).await;

    Ok(Some(Feed { body, etag }))
}

impl From<db::items::CalendarFeedItemRow> for FeedItem {
    fn from(row: db::items::CalendarFeedItemRow) -> Self {
        Self {
            id: row.id,
            list_name: row.list_name,
            title: row.title,
            description: row.description,
            completed: row.completed,
            start_date: row.start_date,
            start_time: row.start_time,
            deadline: row.deadline,
            deadline_time: row.deadline_time,
            hard_deadline: row.hard_deadline,
            updated_at: row.updated_at,
        }
    }
}

/// A token is exactly [`TOKEN_LEN`] characters of the unpadded base64url alphabet.
fn is_well_formed(token: &str) -> bool {
    token.len() == TOKEN_LEN
        && token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || BASE64URL_EXTRA.contains(&b))
}

/// Generates a fresh secret: 32 random bytes as unpadded base64url (43 chars).
fn generate_token() -> Result<String, DomainError> {
    let mut bytes = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(|e| DomainError::Internal(format!("getrandom: {e}")))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

/// Lowercase hex SHA-256 of the token — the only representation that is stored.
fn hash_token(token: &str) -> String {
    hex_encode(Sha256::digest(token.as_bytes()).as_slice())
}

/// First [`ETAG_HEX_CHARS`] hex characters of the body digest.
fn etag_digest(body: &str) -> String {
    hex_encode(Sha256::digest(body.as_bytes()).as_slice())
        .chars()
        .take(ETAG_HEX_CHARS)
        .collect()
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

/// The user's configured timezone, falling back to UTC when unset or unknown.
async fn resolve_timezone(pool: &SqlitePool, user_id: &str) -> Result<Tz, DomainError> {
    let tz_str = db::preferences::get_timezone(pool, user_id).await?;
    Ok(tz_str.parse().unwrap_or(chrono_tz::UTC))
}
