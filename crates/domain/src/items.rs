use crate::{DomainError, rules};
use kartoteka_db as db;
use kartoteka_shared::types::FlexDate;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
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
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub tags: Vec<ItemTag>,
    #[serde(default)]
    pub comments: Vec<crate::comments::Comment>,
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateItemRequest {
    pub title: String,
    pub description: Option<String>,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<String>,
    pub start_time: Option<String>,
    pub deadline: Option<String>,
    pub deadline_time: Option<String>,
    pub hard_deadline: Option<String>,
    pub estimated_duration: Option<i32>,
}

/// All fields optional. None = don't update. For nullable: Some(None) = clear to NULL.
#[derive(Debug, Deserialize)]
pub struct UpdateItemRequest {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub completed: Option<bool>,
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
    rules::items::validate_title(&req.title)?;
    DomainError::ensure_unique(
        db::items::find_id_by_title_in_list(pool, list_id, &req.title, None).await?,
        "item",
    )?;
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
    rules::items::validate_features(&ctx.features, has_dates, has_quantity)?;

    // Phase 3: WRITE
    let row = db::items::insert(
        pool,
        &db::items::InsertItemInput {
            id: Uuid::new_v4().to_string(),
            list_id: list_id.to_string(),
            position: i32::try_from(ctx.next_position).unwrap_or(i32::MAX),
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
        },
    )
    .await?;

    Ok(row.into())
}

#[tracing::instrument(skip(pool))]
pub async fn update(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
    req: &UpdateItemRequest,
) -> Result<Option<Item>, DomainError> {
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
    crate::locations::ensure_owned(pool, &req.location_id, user_id).await?;

    // Phase 3: WRITE
    let input = db::items::UpdateItemInput {
        title: req.title.clone(),
        description: req.description.clone(),
        completed: req.completed,
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
    };
    let found = db::items::update(pool, id, user_id, &input).await?;
    if !found {
        return Ok(None);
    }
    let item = match db::items::get_one(pool, id, user_id).await?.map(Item::from) {
        Some(i) => i,
        None => return Ok(None),
    };
    // Auto-complete check
    if !item.completed {
        if let (Some(actual), Some(qty)) = (item.actual_quantity, item.quantity) {
            if rules::items::should_auto_complete(actual, qty) {
                db::items::set_completed(pool, id, true).await?;
                return Ok(db::items::get_one(pool, id, user_id).await?.map(Item::from));
            }
        }
    }
    Ok(Some(item))
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
) -> Result<Option<Item>, DomainError> {
    // Phase 1 READ: check current state
    let item = match db::items::get_one(pool, id, user_id).await? {
        Some(row) => Item::from(row),
        None => return Ok(None),
    };

    // Feature gate
    let features = db::lists::get_feature_names(pool, &item.list_id).await?;
    if !features.iter().any(|f| f == "checklist") {
        return Err(DomainError::FeatureRequired("checklist"));
    }

    // Phase 2 THINK: only check blockers when completing (false → true)
    if !item.completed {
        let blockers = db::relations::get_unresolved_blockers(pool, id).await?;
        crate::rules::items::validate_can_complete(blockers.len())?;
    }

    // Phase 3 WRITE: toggle
    let found = db::items::toggle_complete(pool, id, user_id).await?;
    if !found {
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

#[tracing::instrument(skip(pool))]
pub async fn by_date(
    pool: &SqlitePool,
    user_id: &str,
    date: &str,
) -> Result<Vec<Item>, DomainError> {
    let resolved = if date == "today" {
        let tz_str = db::preferences::get_timezone(pool, user_id).await?;
        let tz: chrono_tz::Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
        chrono::Utc::now()
            .with_timezone(&tz)
            .date_naive()
            .format("%Y-%m-%d")
            .to_string()
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
    let tz_str = db::preferences::get_timezone(pool, user_id).await?;
    let tz: chrono_tz::Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
    let today = chrono::Utc::now()
        .with_timezone(&tz)
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    overdue_for(pool, user_id, &today).await
}

// ── Integration tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
