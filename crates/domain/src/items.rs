use crate::{DomainError, rules};
use chrono::NaiveDate;
use kartoteka_db as db;
use kartoteka_shared::types::{FlexDate, RecurrenceMode, RecurrenceRule};
use serde::{Deserialize, Serialize};
use sqlx::{SqliteConnection, SqlitePool};
use std::collections::HashMap;
use uuid::Uuid;

// ── Public domain types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemTag {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub parent_tag_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub description: Option<String>,
    pub completed: bool,
    /// "todo" | "in_progress" | "done"; `done` exactly when `completed`.
    pub status: String,
    /// `lucide:<name>`, `emoji:<e>` or a legacy bare emoji.
    pub icon: Option<String>,
    pub position: i32,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<FlexDate>,
    pub start_time: Option<String>,
    pub deadline: Option<FlexDate>,
    pub deadline_time: Option<String>,
    pub hard_deadline: Option<FlexDate>,
    pub estimated_duration: Option<i32>,
    pub location_id: Option<String>,
    pub recurrence_rule: Option<RecurrenceRule>,
    pub recurrence_mode: Option<RecurrenceMode>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub tags: Vec<ItemTag>,
    #[serde(default)]
    pub comments: Vec<crate::comments::Comment>,
}

/// Result of an operation that may complete an item: the item itself plus the
/// next occurrence its recurrence rule spawned.
#[derive(Debug, Clone, Serialize)]
pub struct ItemChange {
    pub item: Item,
    pub spawned: Option<Item>,
}

/// Map onto the shared DTO the server functions serialize.
impl From<ItemChange> for kartoteka_shared::types::ItemChange {
    fn from(change: ItemChange) -> Self {
        Self {
            item: change.item.into(),
            spawned: change.spawned.map(Into::into),
        }
    }
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateItemRequest {
    pub title: String,
    pub description: Option<String>,
    /// `lucide:<name>`, `emoji:<e>` or a legacy bare emoji.
    pub icon: Option<String>,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<String>,
    pub start_time: Option<String>,
    pub deadline: Option<String>,
    pub deadline_time: Option<String>,
    pub hard_deadline: Option<String>,
    pub estimated_duration: Option<i32>,
    /// Compact rule form, e.g. `"7d"` / `"2w"` / `"1m"`.
    pub recurrence_rule: Option<String>,
    /// `"fixed"` (default when a rule is set) or `"floating"`.
    pub recurrence_mode: Option<String>,
}

/// All fields optional. None = don't update. For nullable: Some(None) = clear to NULL.
#[derive(Debug, Default, Deserialize)]
pub struct UpdateItemRequest {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    /// Drives `status` too (`true` → done, `false` → todo unless in_progress).
    pub completed: Option<bool>,
    /// None = keep, Some(None) = clear.
    pub icon: Option<Option<String>>,
    pub quantity: Option<Option<i32>>,
    pub actual_quantity: Option<Option<i32>>,
    pub unit: Option<Option<String>>,
    pub start_date: Option<Option<String>>,
    pub start_time: Option<Option<String>>,
    pub deadline: Option<Option<String>>,
    pub deadline_time: Option<Option<String>>,
    pub hard_deadline: Option<Option<String>>,
    pub estimated_duration: Option<Option<i32>>,
    pub location_id: Option<Option<String>>,
    pub recurrence_rule: Option<Option<String>>,
    pub recurrence_mode: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct MoveItemRequest {
    pub position: i32,
    pub list_id: Option<String>, // move to different list
}

// ── Container item type ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ContainerItem {
    pub item: Item,
    pub list_name: String,
    pub container_id: String,
    pub container_name: String,
}

// ── Conversion from db row ────────────────────────────────────────────────────

impl From<db::types::ItemRow> for Item {
    fn from(row: db::types::ItemRow) -> Self {
        Self {
            id: row.id,
            list_id: row.list_id,
            title: row.title,
            description: row.description,
            completed: row.completed,
            status: row.status,
            icon: row.icon,
            position: row.position,
            quantity: row.quantity,
            actual_quantity: row.actual_quantity,
            unit: row.unit,
            start_date: row.start_date,
            start_time: row.start_time,
            deadline: row.deadline,
            deadline_time: row.deadline_time,
            hard_deadline: row.hard_deadline,
            estimated_duration: row.estimated_duration,
            location_id: row.location_id,
            recurrence_rule: row.recurrence_rule,
            recurrence_mode: row.recurrence_mode,
            created_at: row.created_at,
            updated_at: row.updated_at,
            tags: vec![],
            comments: vec![],
        }
    }
}

/// The single place mapping a domain item to its serialized counterpart.
impl From<Item> for kartoteka_shared::types::Item {
    fn from(item: Item) -> Self {
        Self {
            id: item.id,
            list_id: item.list_id,
            title: item.title,
            description: item.description,
            completed: item.completed,
            status: item.status,
            icon: item.icon,
            position: item.position,
            quantity: item.quantity,
            actual_quantity: item.actual_quantity,
            unit: item.unit,
            start_date: item.start_date,
            start_time: item.start_time,
            deadline: item.deadline,
            deadline_time: item.deadline_time,
            hard_deadline: item.hard_deadline,
            estimated_duration: item.estimated_duration,
            location_id: item.location_id,
            recurrence_rule: item.recurrence_rule,
            recurrence_mode: item.recurrence_mode,
            created_at: item.created_at,
            updated_at: item.updated_at,
        }
    }
}

fn tag_row_to_item_tag(r: db::types::TagRow) -> ItemTag {
    ItemTag {
        id: r.id,
        name: r.name,
        color: r.color,
        parent_tag_id: r.parent_tag_id,
    }
}

fn list_item_tag_row_to_item_tag(r: db::tags::ListItemTagRow) -> ItemTag {
    ItemTag {
        id: r.id,
        name: r.name,
        color: r.color,
        parent_tag_id: r.parent_tag_id,
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn effective_date_field(
    current: &Option<FlexDate>,
    req: Option<&Option<String>>,
) -> Option<String> {
    match req {
        None => current.as_ref().map(|f| f.to_string()),
        Some(None) => None,
        Some(Some(s)) => Some(s.clone()),
    }
}

fn effective_str_field<'a>(
    current: Option<&'a str>,
    req: Option<&'a Option<String>>,
) -> Option<&'a str> {
    match req {
        None => current,
        Some(None) => None,
        Some(Some(s)) => Some(s.as_str()),
    }
}

/// The date fields of raw request strings, ignoring unparsable ones (already
/// rejected by [`rules::items::validate_item_dates`]).
fn item_dates_of(
    start_date: Option<&str>,
    deadline: Option<&str>,
    hard_deadline: Option<&str>,
) -> rules::recurrence::ItemDates {
    let parse = |s: &str| s.parse::<FlexDate>().ok();
    rules::recurrence::ItemDates {
        start_date: start_date.and_then(parse),
        deadline: deadline.and_then(parse),
        hard_deadline: hard_deadline.and_then(parse),
    }
}

/// Parse a raw rule/mode pair. A rule given without a mode repeats `fixed`.
fn parse_recurrence(
    rule_raw: Option<&str>,
    mode_raw: Option<&str>,
) -> Result<(Option<RecurrenceRule>, Option<RecurrenceMode>), DomainError> {
    let rule = rule_raw.map(rules::recurrence::parse_rule).transpose()?;
    let mode = mode_raw.map(rules::recurrence::parse_mode).transpose()?;
    let mode = match (rule.is_some(), mode) {
        (true, None) => Some(RecurrenceMode::Fixed),
        _ => mode,
    };
    Ok((rule, mode))
}

/// The persisted form of a parsed rule/mode pair.
fn recurrence_strings(
    recurrence: (Option<RecurrenceRule>, Option<RecurrenceMode>),
) -> (Option<String>, Option<String>) {
    (
        recurrence.0.map(|rule| rule.to_string()),
        recurrence.1.map(|mode| mode.to_string()),
    )
}

/// Raw recurrence strings after applying an update patch: `None` keeps the
/// current value, `Some(None)` clears it. Clearing the rule clears the mode
/// with it, so a rule-less item never keeps a dangling mode.
fn effective_recurrence_raw(
    current: &Item,
    req: &UpdateItemRequest,
) -> (Option<String>, Option<String>) {
    let rule = match &req.recurrence_rule {
        None => current.recurrence_rule.map(|rule| rule.to_string()),
        Some(None) => None,
        Some(Some(raw)) => Some(raw.clone()),
    };
    let mode = match &req.recurrence_mode {
        Some(value) => value.clone(),
        None if rule.is_none() => None,
        None => current.recurrence_mode.map(|mode| mode.to_string()),
    };
    (rule, mode)
}

/// True when the item repeats from its completion date, i.e. the next
/// occurrence needs the user's current date.
fn is_floating(item: &Item) -> bool {
    item.recurrence_rule.is_some()
        && item.recurrence_mode.unwrap_or_default() == RecurrenceMode::Floating
}

/// The date a spawn is anchored to: the user's `today` for a floating rule.
/// Fixed rules ignore it, so their timezone lookup is skipped.
async fn completion_day(
    pool: &SqlitePool,
    user_id: &str,
    item: &Item,
) -> Result<NaiveDate, DomainError> {
    if !is_floating(item) {
        return Ok(chrono::Utc::now().date_naive());
    }
    let today = user_today(pool, user_id).await?;
    NaiveDate::parse_from_str(&today, "%Y-%m-%d")
        .map_err(|_| DomainError::Internal(format!("invalid today date: {today}")))
}

// ── Orchestration ─────────────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn list_for_list(
    pool: &SqlitePool,
    list_id: &str,
    user_id: &str,
) -> Result<Vec<Item>, DomainError> {
    let (rows, tag_rows, comment_rows) = tokio::try_join!(
        db::items::list_for_list(pool, list_id, user_id),
        db::tags::get_tags_for_list_items(pool, list_id, user_id),
        db::comments::get_comments_for_list_items(pool, list_id, user_id),
    )?;
    let mut tags_map: HashMap<String, Vec<ItemTag>> = HashMap::new();
    for r in tag_rows {
        tags_map
            .entry(r.item_id.clone())
            .or_default()
            .push(list_item_tag_row_to_item_tag(r));
    }
    let mut comments_map: HashMap<String, Vec<crate::comments::Comment>> = HashMap::new();
    for r in comment_rows {
        comments_map
            .entry(r.entity_id.clone())
            .or_default()
            .push(crate::comments::row_to_comment(r));
    }
    Ok(rows
        .into_iter()
        .map(|row| {
            let mut item = Item::from(row);
            item.tags = tags_map.remove(&item.id).unwrap_or_default();
            item.comments = comments_map.remove(&item.id).unwrap_or_default();
            item
        })
        .collect())
}

#[tracing::instrument(skip(pool))]
pub async fn list_for_container(
    pool: &SqlitePool,
    container_id: &str,
    user_id: &str,
    limit: u32,
    offset: usize,
    recursive: bool,
) -> Result<Vec<ContainerItem>, DomainError> {
    let rows = db::items::list_for_container(pool, container_id, user_id, limit, offset, recursive)
        .await?;
    Ok(rows
        .into_iter()
        .map(|r| ContainerItem {
            item: r.item.into(),
            list_name: r.list_name,
            container_id: r.container_id,
            container_name: r.container_name,
        })
        .collect())
}

#[tracing::instrument(skip(pool))]
pub async fn get_one(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<Item>, DomainError> {
    let (item_opt, tag_rows, comments) = tokio::try_join!(
        async {
            db::items::get_one(pool, id, user_id)
                .await
                .map_err(DomainError::from)
        },
        async {
            db::tags::get_tags_for_item(pool, id, user_id)
                .await
                .map_err(DomainError::from)
        },
        crate::comments::list_for_item_unchecked(pool, id),
    )?;
    Ok(item_opt.map(|row| {
        let mut item = Item::from(row);
        item.tags = tag_rows.into_iter().map(tag_row_to_item_tag).collect();
        item.comments = comments;
        item
    }))
}

// ── Create building blocks ────────────────────────────────────────────────────

/// Rules every new item must satisfy, independent of how it is persisted.
///
/// `features` are the feature names enabled on the target list. Uniqueness is
/// deliberately not checked here: [`create`] rejects duplicates while the MCP
/// batch reports them as `existed` in its own dedup pass.
pub fn validate_create_item(
    features: &[String],
    req: &CreateItemRequest,
) -> Result<(), DomainError> {
    rules::items::validate_title(&req.title)?;
    rules::appearance::normalize_icon(req.icon.as_deref())?;
    rules::items::validate_item_dates(
        req.start_date.as_deref(),
        req.start_time.as_deref(),
        req.deadline.as_deref(),
        req.deadline_time.as_deref(),
        req.hard_deadline.as_deref(),
    )?;
    let has_dates =
        req.start_date.is_some() || req.deadline.is_some() || req.hard_deadline.is_some();
    let has_quantity = req.quantity.is_some() || req.unit.is_some();
    rules::items::validate_features(features, has_dates, has_quantity)?;
    let (rule, mode) = parse_recurrence(
        req.recurrence_rule.as_deref(),
        req.recurrence_mode.as_deref(),
    )?;
    rules::recurrence::validate_recurrence(
        rule.as_ref(),
        mode,
        &item_dates_of(
            req.start_date.as_deref(),
            req.deadline.as_deref(),
            req.hard_deadline.as_deref(),
        ),
    )
}

/// Map a validated create request onto the DB insert input.
pub fn new_item_input(
    list_id: &str,
    position: i32,
    req: &CreateItemRequest,
) -> db::items::InsertItemInput {
    // A rule without an explicit mode is stored as `fixed`. Requests are
    // validated first; the fallback never writes an unparsable rule, because
    // such a row would fail to decode on every read of its list.
    let (recurrence_rule, recurrence_mode) = parse_recurrence(
        req.recurrence_rule.as_deref(),
        req.recurrence_mode.as_deref(),
    )
    .map(recurrence_strings)
    .unwrap_or_default();
    db::items::InsertItemInput {
        id: Uuid::new_v4().to_string(),
        list_id: list_id.to_string(),
        position,
        title: req.title.clone(),
        description: req.description.clone(),
        quantity: req.quantity,
        actual_quantity: req.actual_quantity,
        unit: req.unit.clone(),
        start_date: req.start_date.clone(),
        start_time: req.start_time.clone(),
        deadline: req.deadline.clone(),
        deadline_time: req.deadline_time.clone(),
        hard_deadline: req.hard_deadline.clone(),
        estimated_duration: req.estimated_duration,
        // Not part of CreateItemRequest yet; recurrence behaviour is a separate change.
        location_id: None,
        recurrence_rule,
        recurrence_mode,
        // The length rule lives in `validate_create_item`; only trimming and
        // "empty means no icon" are applied here.
        icon: req
            .icon
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
    }
}

/// Insert an already-validated item inside a caller-owned transaction.
pub async fn insert_item_in(
    conn: &mut SqliteConnection,
    input: &db::items::InsertItemInput,
) -> Result<Item, DomainError> {
    Ok(db::items::insert_in_tx(conn, input).await?.into())
}

#[tracing::instrument(skip(pool))]
pub async fn create(
    pool: &SqlitePool,
    user_id: &str,
    list_id: &str,
    req: &CreateItemRequest,
) -> Result<Item, DomainError> {
    // Phase 1: READ
    let ctx = db::lists::get_create_item_context(pool, list_id, user_id)
        .await?
        .ok_or(DomainError::NotFound("list"))?;

    // Phase 2: THINK
    validate_create_item(&ctx.features, req)?;
    DomainError::ensure_unique(
        db::items::find_id_by_title_in_list(pool, list_id, &req.title, None).await?,
        "item",
    )?;

    // Phase 3: WRITE
    let input = new_item_input(
        list_id,
        i32::try_from(ctx.next_position).unwrap_or(i32::MAX),
        req,
    );
    let mut tx = pool.begin().await.map_err(db::DbError::Sqlx)?;
    let item = insert_item_in(&mut tx, &input).await?;
    tx.commit().await.map_err(db::DbError::Sqlx)?;

    Ok(item)
}

#[tracing::instrument(skip(pool))]
pub async fn update(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
    req: &UpdateItemRequest,
) -> Result<Option<ItemChange>, DomainError> {
    // Phase 1: READ
    let current = match db::items::get_one(pool, id, user_id).await?.map(Item::from) {
        Some(i) => i,
        None => return Ok(None),
    };

    // Phase 2: THINK
    if let Some(title) = &req.title {
        rules::items::validate_title(title)?;
        DomainError::ensure_unique(
            db::items::find_id_by_title_in_list(pool, &current.list_id, title, Some(id)).await?,
            "item",
        )?;
    }
    let eff_start_date = effective_date_field(&current.start_date, req.start_date.as_ref());
    let eff_start_time =
        effective_str_field(current.start_time.as_deref(), req.start_time.as_ref());
    let eff_deadline = effective_date_field(&current.deadline, req.deadline.as_ref());
    let eff_deadline_time =
        effective_str_field(current.deadline_time.as_deref(), req.deadline_time.as_ref());
    let eff_hard_deadline =
        effective_date_field(&current.hard_deadline, req.hard_deadline.as_ref());
    rules::items::validate_item_dates(
        eff_start_date.as_deref(),
        eff_start_time,
        eff_deadline.as_deref(),
        eff_deadline_time,
        eff_hard_deadline.as_deref(),
    )?;
    let (eff_rule_raw, eff_mode_raw) = effective_recurrence_raw(&current, req);
    let (eff_rule, eff_mode) = parse_recurrence(eff_rule_raw.as_deref(), eff_mode_raw.as_deref())?;
    rules::recurrence::validate_recurrence(
        eff_rule.as_ref(),
        eff_mode,
        &item_dates_of(
            eff_start_date.as_deref(),
            eff_deadline.as_deref(),
            eff_hard_deadline.as_deref(),
        ),
    )?;
    crate::locations::ensure_owned(pool, &req.location_id, user_id).await?;
    let icon = rules::appearance::normalize_icon(req.icon.as_ref().and_then(|v| v.as_deref()))?;
    // A caller-provided icon only replaces the stored one when `icon` was given
    // (`Some`); `None` means "leave it alone".
    let icon = req.icon.as_ref().map(|_| icon);

    // The false → true transition (explicit or auto-complete) drives the spawn.
    // It is decided from the pre-write state, before the transaction opens.
    let eff_actual = req.actual_quantity.unwrap_or(current.actual_quantity);
    let eff_quantity = req.quantity.unwrap_or(current.quantity);
    let auto_complete = matches!(
        (eff_actual, eff_quantity),
        (Some(actual), Some(quantity)) if rules::items::should_auto_complete(actual, quantity)
    );
    let completing = !current.completed && (req.completed == Some(true) || auto_complete);
    let today = if completing {
        completion_day(pool, user_id, &current).await?
    } else {
        chrono::Utc::now().date_naive()
    };

    // Phase 3: WRITE — the update, the spawn and the tag copy commit together
    let (recurrence_rule, recurrence_mode) = recurrence_strings((eff_rule, eff_mode));
    let input = db::items::UpdateItemInput {
        title: req.title.clone(),
        description: req.description.clone(),
        completed: if completing {
            Some(true)
        } else {
            req.completed
        },
        icon,
        quantity: req.quantity,
        actual_quantity: req.actual_quantity,
        unit: req.unit.clone(),
        start_date: req.start_date.clone(),
        start_time: req.start_time.clone(),
        deadline: req.deadline.clone(),
        deadline_time: req.deadline_time.clone(),
        hard_deadline: req.hard_deadline.clone(),
        estimated_duration: req.estimated_duration,
        location_id: req.location_id.clone(),
        recurrence_rule: Some(recurrence_rule),
        recurrence_mode: Some(recurrence_mode),
    };
    let mut tx = pool.begin().await.map_err(db::DbError::Sqlx)?;
    if !db::items::update_in_tx(&mut tx, id, user_id, &input).await? {
        return Ok(None);
    }
    let spawned_id = if completing {
        // Spawn from the row as written, so a date or quantity patched by this
        // same request is what the next occurrence repeats from.
        let written = db::items::get_one_in_tx(&mut tx, id, user_id)
            .await?
            .map(Item::from)
            .ok_or(DomainError::NotFound("item"))?;
        spawn_next_in(&mut tx, user_id, &written, today).await?
    } else {
        None
    };
    tx.commit().await.map_err(db::DbError::Sqlx)?;

    Ok(Some(item_change(pool, user_id, id, spawned_id).await?))
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, user_id: &str, id: &str) -> Result<bool, DomainError> {
    Ok(db::items::delete(pool, id, user_id).await?)
}

#[tracing::instrument(skip(pool))]
pub async fn toggle_complete(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
) -> Result<Option<ItemChange>, DomainError> {
    // Phase 1 READ: check current state
    let current = match db::items::get_one(pool, id, user_id).await? {
        Some(row) => Item::from(row),
        None => return Ok(None),
    };

    // Feature gate
    let features = db::lists::get_feature_names(pool, &current.list_id).await?;
    if !features.iter().any(|f| f == "checklist") {
        return Err(DomainError::FeatureRequired("checklist"));
    }

    // Phase 2 THINK: only check blockers when completing (false → true)
    let completing = !current.completed;
    if completing {
        let blockers = db::relations::get_unresolved_blockers(pool, id).await?;
        crate::rules::items::validate_can_complete(blockers.len())?;
    }
    let today = if completing {
        completion_day(pool, user_id, &current).await?
    } else {
        chrono::Utc::now().date_naive()
    };

    // Phase 3 WRITE: the flip, the spawn and the tag copy commit together
    let mut tx = pool.begin().await.map_err(db::DbError::Sqlx)?;
    if !db::items::set_completed_in_tx(&mut tx, id, user_id, completing).await? {
        return Ok(None);
    }
    let spawned_id = if completing {
        let written = db::items::get_one_in_tx(&mut tx, id, user_id)
            .await?
            .map(Item::from)
            .ok_or(DomainError::NotFound("item"))?;
        spawn_next_in(&mut tx, user_id, &written, today).await?
    } else {
        None
    };
    tx.commit().await.map_err(db::DbError::Sqlx)?;

    Ok(Some(item_change(pool, user_id, id, spawned_id).await?))
}

/// Insert the next occurrence of `completed` and move the rule off it, inside
/// the caller's transaction. Returns the new item id, or `None` when the item
/// does not recur.
///
/// The spawned item copies the plan (title, description, quantity, unit, times,
/// duration, location, tags and the rule itself) with the dates shifted by
/// [`rules::recurrence::next_dates`]; relations and comments stay behind.
async fn spawn_next_in(
    conn: &mut SqliteConnection,
    user_id: &str,
    completed: &Item,
    today: NaiveDate,
) -> Result<Option<String>, DomainError> {
    let Some(rule) = completed.recurrence_rule else {
        return Ok(None);
    };
    let mode = completed.recurrence_mode.unwrap_or_default();
    let next = rules::recurrence::next_dates(
        &rules::recurrence::ItemDates {
            start_date: completed.start_date.clone(),
            deadline: completed.deadline.clone(),
            hard_deadline: completed.hard_deadline.clone(),
        },
        &rule,
        mode,
        today,
    );
    let input = db::items::InsertItemInput {
        id: Uuid::new_v4().to_string(),
        list_id: completed.list_id.clone(),
        position: db::items::next_position_in_tx(&mut *conn, &completed.list_id).await?,
        title: completed.title.clone(),
        description: completed.description.clone(),
        quantity: completed.quantity,
        actual_quantity: None,
        unit: completed.unit.clone(),
        start_date: next.start_date.as_ref().map(ToString::to_string),
        start_time: completed.start_time.clone(),
        deadline: next.deadline.as_ref().map(ToString::to_string),
        deadline_time: completed.deadline_time.clone(),
        hard_deadline: next.hard_deadline.as_ref().map(ToString::to_string),
        estimated_duration: completed.estimated_duration,
        location_id: completed.location_id.clone(),
        recurrence_rule: Some(rule.to_string()),
        recurrence_mode: Some(mode.to_string()),
        icon: completed.icon.clone(),
    };
    let spawned = db::items::insert_in_tx(&mut *conn, &input).await?;
    db::tags::copy_item_tags_in_tx(&mut *conn, &completed.id, &spawned.id).await?;

    // The rule moves forward: the completed original stops recurring, so
    // un-completing it and completing it again spawns nothing.
    let clear_rule = db::items::UpdateItemInput {
        title: None,
        description: None,
        completed: None,
        quantity: None,
        actual_quantity: None,
        unit: None,
        start_date: None,
        start_time: None,
        deadline: None,
        deadline_time: None,
        hard_deadline: None,
        estimated_duration: None,
        location_id: None,
        recurrence_rule: Some(None),
        recurrence_mode: Some(None),
        icon: None,
    };
    db::items::update_in_tx(&mut *conn, &completed.id, user_id, &clear_rule).await?;

    Ok(Some(spawned.id))
}

/// Re-read the original item and the spawned occurrence after the transaction
/// committed, so both carry their tags (and the original its comments).
async fn item_change(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
    spawned_id: Option<String>,
) -> Result<ItemChange, DomainError> {
    let item = get_one(pool, id, user_id)
        .await?
        .ok_or(DomainError::NotFound("item"))?;
    let spawned = match spawned_id {
        Some(spawned_id) => Some(
            get_one(pool, &spawned_id, user_id)
                .await?
                .ok_or(DomainError::NotFound("item"))?,
        ),
        None => None,
    };
    Ok(ItemChange { item, spawned })
}

/// Set the three-state status of an item.
///
/// `done` goes through the same gate as completing an item: the list must be a
/// checklist and no unresolved blocker may remain (see
/// [`rules::items::validate_can_complete`]). `todo` / `in_progress` always clear
/// `completed`, so `completed == (status == 'done')` keeps holding. Unknown
/// status values are rejected before anything is written.
#[tracing::instrument(skip(pool), fields(item_id = %id, status = %status))]
pub async fn set_status(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
    status: &str,
) -> Result<Option<Item>, DomainError> {
    // Phase 1 READ: current state + feature gate
    rules::items::validate_status(status)?;
    let item = match db::items::get_one(pool, id, user_id).await? {
        Some(row) => Item::from(row),
        None => return Ok(None),
    };

    // Phase 2 THINK: becoming done is completing the item — reuse the toggle
    // path so the checklist gate, blocker check and recurrence spawn all apply.
    if status == "done" && !item.completed {
        return Ok(toggle_complete(pool, user_id, id).await?.map(|c| c.item));
    }

    // Phase 3 WRITE: `completed` is derived in the same statement
    if !db::items::set_status(pool, id, user_id, status).await? {
        return Ok(None);
    }
    Ok(db::items::get_one(pool, id, user_id).await?.map(Item::from))
}

#[tracing::instrument(skip(pool))]
pub async fn move_item(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
    req: &MoveItemRequest,
) -> Result<Option<Item>, DomainError> {
    // Phase 1 READ: get current item + validate target list if provided
    let current = match db::items::get_one(pool, id, user_id).await?.map(Item::from) {
        Some(i) => i,
        None => return Ok(None),
    };
    if let Some(target_list_id) = &req.list_id {
        if db::lists::get_one(pool, target_list_id, user_id)
            .await?
            .is_none()
        {
            return Err(DomainError::NotFound("list"));
        }
    }
    // Phase 2 THINK: duplicate check in target list
    let target_list = req.list_id.as_deref().unwrap_or(&current.list_id);
    DomainError::ensure_unique(
        db::items::find_id_by_title_in_list(pool, target_list, &current.title, Some(id)).await?,
        "item",
    )?;
    // Phase 3 WRITE
    let found =
        db::items::move_item(pool, id, user_id, req.position, req.list_id.as_deref()).await?;
    if !found {
        return Ok(None);
    }
    Ok(db::items::get_one(pool, id, user_id).await?.map(Item::from))
}

/// Rewrite item positions inside one list, atomically.
///
/// Every id in `ordered_ids` must be an item of `list_id` owned by `user_id`,
/// otherwise the call fails with [`DomainError::NotFound`] before anything is
/// written (no partial reorder). Positions of ids that are not mentioned keep
/// their current value.
#[tracing::instrument(skip(pool, ordered_ids), fields(list_id = %list_id, count = ordered_ids.len()))]
pub async fn reorder(
    pool: &SqlitePool,
    user_id: &str,
    list_id: &str,
    ordered_ids: &[String],
) -> Result<(), DomainError> {
    if db::lists::get_one(pool, list_id, user_id).await?.is_none() {
        return Err(DomainError::NotFound("list"));
    }
    let scope = db::items::ordered_ids_in_list(pool, user_id, list_id).await?;
    DomainError::ensure_ids_in_scope(&scope, ordered_ids, "item")?;

    let mut tx = pool.begin().await.map_err(db::DbError::Sqlx)?;
    for (pos, id) in ordered_ids.iter().enumerate() {
        if !db::items::set_position_in_tx(&mut tx, id, user_id, list_id, pos as i32).await? {
            // Cannot happen after the scope check above; keep the guard so a
            // concurrent change can never produce a silent partial write.
            return Err(DomainError::NotFound("item"));
        }
    }
    tx.commit().await.map_err(db::DbError::Sqlx)?;
    Ok(())
}

/// Move `item_id` into `target_list_id` right before `before_item_id`
/// (`None` = append), then rewrite the target list positions in the same
/// transaction.
///
/// The target list must belong to `user_id` and the item title must stay unique
/// inside it (same rule as [`move_item`]).
#[tracing::instrument(skip(pool, before_item_id), fields(item_id = %item_id, target_list_id = %target_list_id))]
pub async fn place(
    pool: &SqlitePool,
    user_id: &str,
    item_id: &str,
    target_list_id: &str,
    before_item_id: Option<&str>,
) -> Result<(), DomainError> {
    // Phase 1 READ: item (owner-scoped) + target list ownership
    let current = db::items::get_one(pool, item_id, user_id)
        .await?
        .map(Item::from)
        .ok_or(DomainError::NotFound("item"))?;
    if db::lists::get_one(pool, target_list_id, user_id)
        .await?
        .is_none()
    {
        return Err(DomainError::NotFound("list"));
    }
    // Phase 2 THINK: duplicate-title rule of the target list
    DomainError::ensure_unique(
        db::items::find_id_by_title_in_list(pool, target_list_id, &current.title, Some(item_id))
            .await?,
        "item",
    )?;

    let mut target_ids = db::items::ordered_ids_in_list(pool, user_id, target_list_id).await?;
    target_ids.retain(|id| id != item_id);
    let insert_at = match before_item_id.filter(|before| *before != item_id) {
        Some(before) => target_ids
            .iter()
            .position(|id| id == before)
            .ok_or(DomainError::NotFound("item"))?,
        None => target_ids.len(),
    };
    target_ids.insert(insert_at, item_id.to_string());

    // Phase 3 WRITE: one transaction — move first, then rewrite positions
    let mut tx = pool.begin().await.map_err(db::DbError::Sqlx)?;
    if !db::items::move_item_in_tx(&mut tx, item_id, user_id, 0, Some(target_list_id)).await? {
        return Err(DomainError::NotFound("item"));
    }
    for (pos, id) in target_ids.iter().enumerate() {
        if !db::items::set_position_in_tx(&mut tx, id, user_id, target_list_id, pos as i32).await? {
            return Err(DomainError::NotFound("item"));
        }
    }
    tx.commit().await.map_err(db::DbError::Sqlx)?;
    Ok(())
}

/// Format `now` as a "YYYY-MM-DD" date in `tz_str`, falling back to UTC when the
/// timezone name is invalid. Kept pure so timezone-boundary behaviour is testable.
pub fn date_in_tz(now: chrono::DateTime<chrono::Utc>, tz_str: &str) -> String {
    let tz: chrono_tz::Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
    now.with_timezone(&tz)
        .date_naive()
        .format("%Y-%m-%d")
        .to_string()
}

/// Today's date ("YYYY-MM-DD") in the user's configured timezone.
///
/// All "today"-based logic (today view, overdue, list grouping) must go through
/// this helper so it agrees on the user's local date instead of UTC.
#[tracing::instrument(skip(pool))]
pub async fn user_today(pool: &SqlitePool, user_id: &str) -> Result<String, DomainError> {
    let tz_str = db::preferences::get_timezone(pool, user_id).await?;
    Ok(date_in_tz(chrono::Utc::now(), &tz_str))
}

#[tracing::instrument(skip(pool))]
pub async fn by_date(
    pool: &SqlitePool,
    user_id: &str,
    date: &str,
) -> Result<Vec<Item>, DomainError> {
    let resolved = if date == "today" {
        user_today(pool, user_id).await?
    } else {
        date.to_string()
    };
    let rows = db::items::by_date(pool, user_id, &resolved).await?;
    Ok(rows.into_iter().map(Item::from).collect())
}

#[tracing::instrument(skip(pool))]
pub async fn calendar(
    pool: &SqlitePool,
    user_id: &str,
    year_month: &str,
) -> Result<Vec<Item>, DomainError> {
    let rows = db::items::calendar(pool, user_id, year_month).await?;
    Ok(rows.into_iter().map(Item::from).collect())
}

#[tracing::instrument(skip(pool))]
pub async fn list_all_for_user(pool: &SqlitePool, user_id: &str) -> Result<Vec<Item>, DomainError> {
    let rows = db::items::list_all_for_user(pool, user_id).await?;
    Ok(rows.into_iter().map(Item::from).collect())
}

/// Returns incomplete items with `deadline` strictly before `today` (YYYY-MM-DD).
/// Use this from MCP handlers where today is pre-computed from the user's timezone.
#[tracing::instrument(skip(pool))]
pub async fn overdue_for(
    pool: &SqlitePool,
    user_id: &str,
    today: &str,
) -> Result<Vec<Item>, DomainError> {
    let rows = db::items::overdue(pool, user_id, today).await?;
    Ok(rows.into_iter().map(Item::from).collect())
}

/// Returns incomplete items with `deadline` strictly before today in the user's timezone.
/// Hard deadlines are intentionally excluded — overdue means a missed `deadline` date.
#[tracing::instrument(skip(pool))]
pub async fn overdue(pool: &SqlitePool, user_id: &str) -> Result<Vec<Item>, DomainError> {
    let today = user_today(pool, user_id).await?;
    overdue_for(pool, user_id, &today).await
}

// ── Integration tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
