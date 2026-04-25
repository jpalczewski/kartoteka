#![allow(clippy::too_many_arguments)]
use kartoteka_shared::types::{
    CalendarMonthData, CalendarWeekDay, DateItem, Item, ListData, TodayData,
};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    crate::server_fns::home::{domain_list_to_shared, domain_tag_to_shared},
    crate::server_fns::utils::format_datetime_in_tz,
    axum_login::AuthSession,
    kartoteka_auth::KartotekaBackend,
    kartoteka_db as db, kartoteka_domain as domain,
    kartoteka_shared::types::ItemTagLink,
    sqlx::SqlitePool,
};

/// Convert domain::items::Item to shared::types::Item.
#[cfg(feature = "ssr")]
pub(crate) fn domain_item_to_shared(item: domain::items::Item) -> Item {
    Item {
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
        created_at: item.created_at,
        updated_at: item.updated_at,
    }
}

/// Fetch list header + items + direct sublists in one call.
#[server(prefix = "/leptos")]
pub async fn get_list_data(list_id: String) -> Result<ListData, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;

    macro_rules! sfn_err {
        ($e:expr) => {
            ServerFnError::new($e.to_string())
        };
    }
    let (list_res, items_res, sublists_res, settings_res, tag_links_res, all_tags_res) = tokio::join!(
        async {
            domain::lists::get_one(&pool, &list_id, &user.id)
                .await
                .map_err(|e| sfn_err!(e))
        },
        async {
            domain::items::list_for_list(&pool, &list_id, &user.id)
                .await
                .map_err(|e| sfn_err!(e))
        },
        async {
            domain::lists::sublists(&pool, &list_id, &user.id)
                .await
                .map_err(|e| sfn_err!(e))
        },
        async {
            domain::settings::list_all(&pool, &user.id)
                .await
                .map_err(|e| sfn_err!(e))
        },
        async {
            db::tags::get_item_tags_for_list(&pool, &list_id, &user.id)
                .await
                .map_err(|e| sfn_err!(e))
        },
        async {
            domain::tags::list_all(&pool, &user.id)
                .await
                .map_err(|e| sfn_err!(e))
        },
    );
    let list = list_res?.ok_or_else(|| ServerFnError::new("list not found".to_string()))?;
    let items = items_res?;
    let sublists = sublists_res?;
    let settings = settings_res?;
    let raw_tag_links: Vec<(String, String)> = tag_links_res?;
    let all_domain_tags: Vec<domain::tags::Tag> = all_tags_res?;

    let tz = settings
        .iter()
        .find(|s| s.key == "timezone")
        .map(|s| s.value.as_str())
        .unwrap_or("UTC");

    let created_at_local = format_datetime_in_tz(&list.created_at, tz);

    let today_date = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let container_name = if let Some(ref cid) = list.container_id {
        domain::containers::get_one(&pool, cid, &user.id)
            .await
            .ok()
            .map(|c| c.name)
    } else {
        None
    };

    Ok(ListData {
        list: domain_list_to_shared(list),
        items: items.into_iter().map(domain_item_to_shared).collect(),
        sublists: sublists.into_iter().map(domain_list_to_shared).collect(),
        created_at_local,
        item_tag_links: raw_tag_links
            .into_iter()
            .map(|(item_id, tag_id)| ItemTagLink { item_id, tag_id })
            .collect(),
        all_tags: all_domain_tags
            .into_iter()
            .map(domain_tag_to_shared)
            .collect(),
        today_date,
        container_name,
    })
}

/// Add a new item to a list. Optionally include quantity, unit, and date fields.
#[server(prefix = "/leptos")]
pub async fn create_item(
    list_id: String,
    title: String,
    description: Option<String>,
    quantity: Option<i32>,
    unit: Option<String>,
    start_date: Option<String>,
    deadline: Option<String>,
    hard_deadline: Option<String>,
) -> Result<Item, ServerFnError> {
    if title.trim().is_empty() {
        return Err(ServerFnError::new("title cannot be empty".to_string()));
    }
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let desc = description.and_then(|d| if d.trim().is_empty() { None } else { Some(d) });
    let opt_date = |s: Option<String>| s.and_then(empty_as_none);
    let item = domain::items::create(
        &pool,
        &user.id,
        &list_id,
        &domain::items::CreateItemRequest {
            title,
            description: desc,
            quantity,
            actual_quantity: None,
            unit,
            start_date: opt_date(start_date),
            start_time: None,
            deadline: opt_date(deadline),
            deadline_time: None,
            hard_deadline: opt_date(hard_deadline),
            estimated_duration: None,
        },
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(domain_item_to_shared(item))
}

/// Toggle completed state of an item. Returns the updated item.
#[server(prefix = "/leptos")]
pub async fn toggle_item(item_id: String) -> Result<Option<Item>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let result = domain::items::toggle_complete(&pool, &user.id, &item_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(result.map(domain_item_to_shared))
}

/// Rewrite item positions in `list_id` according to `item_ids`. Ids not in
/// the list are ignored; missing ids keep their old position.
#[server(prefix = "/leptos")]
pub async fn reorder_items(list_id: String, item_ids: Vec<String>) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    // Ownership guard — ensure caller owns the list.
    if domain::lists::get_one(&pool, &list_id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .is_none()
    {
        return Err(ServerFnError::new("list not found".to_string()));
    }
    for (pos, id) in item_ids.iter().enumerate() {
        kartoteka_db::items::move_item(&pool, id, &user.id, pos as i32, None)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
    }
    Ok(())
}

/// Cross-list item move: put `item_id` into `target_list_id` right before
/// `before_item_id` (None = append). Rewrites positions on the target list so
/// the new order is stable.
#[server(prefix = "/leptos")]
pub async fn set_item_placement(
    item_id: String,
    target_list_id: String,
    before_item_id: Option<String>,
) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    // Fetch current items of target list (ordered).
    let target_items = domain::items::list_for_list(&pool, &target_list_id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let mut target_ids: Vec<String> = target_items
        .into_iter()
        .map(|it| it.id)
        .filter(|id| id != &item_id)
        .collect();
    let insert_at = match before_item_id.as_deref() {
        Some(before) => target_ids
            .iter()
            .position(|id| id == before)
            .unwrap_or(target_ids.len()),
        None => target_ids.len(),
    };
    target_ids.insert(insert_at, item_id.clone());
    // First move the dragged item into the target list (any position); then
    // rewrite positions so the sequence matches target_ids.
    kartoteka_db::items::move_item(&pool, &item_id, &user.id, 0, Some(&target_list_id))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    for (pos, id) in target_ids.iter().enumerate() {
        kartoteka_db::items::move_item(&pool, id, &user.id, pos as i32, None)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
    }
    Ok(())
}

/// Move an item to a different list (appended at the end of the target list).
#[server(prefix = "/leptos")]
pub async fn move_item(item_id: String, target_list_id: String) -> Result<Item, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let ctx = kartoteka_db::lists::get_create_item_context(&pool, &target_list_id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("target list not found".to_string()))?;
    let req = domain::items::MoveItemRequest {
        position: ctx.next_position as i32,
        list_id: Some(target_list_id),
    };
    let item = domain::items::move_item(&pool, &user.id, &item_id, &req)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("item not found".to_string()))?;
    Ok(domain_item_to_shared(item))
}

/// Delete an item by id.
#[server(prefix = "/leptos")]
pub async fn delete_item(item_id: String) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let deleted = domain::items::delete(&pool, &user.id, &item_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    if !deleted {
        return Err(ServerFnError::new("item not found".to_string()));
    }
    Ok(())
}

/// Increment/decrement the actual (collected) quantity of an item.
/// Auto-completes the item when actual reaches the target quantity.
#[server(prefix = "/leptos")]
pub async fn update_actual_quantity(
    item_id: String,
    new_actual: i32,
) -> Result<Item, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let current = domain::items::get_one(&pool, &item_id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("item not found".to_string()))?;
    let auto_complete = current
        .quantity
        .map(|target| new_actual >= target)
        .unwrap_or(false);
    let req = domain::items::UpdateItemRequest {
        title: None,
        description: None,
        completed: if auto_complete { Some(true) } else { None },
        quantity: None,
        actual_quantity: Some(Some(new_actual)),
        unit: None,
        start_date: None,
        start_time: None,
        deadline: None,
        deadline_time: None,
        hard_deadline: None,
        estimated_duration: None,
    };
    let item = domain::items::update(&pool, &user.id, &item_id, &req)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("item not found".to_string()))?;
    Ok(domain_item_to_shared(item))
}

/// Update quantity, actual_quantity and unit of an item.
/// Pass `unit` as empty string to clear it.
#[server(prefix = "/leptos")]
pub async fn update_item_quantity(
    item_id: String,
    quantity: Option<i32>,
    actual_quantity: Option<i32>,
    unit: String,
) -> Result<Item, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let unit_val = if unit.trim().is_empty() {
        None
    } else {
        Some(unit)
    };
    let req = domain::items::UpdateItemRequest {
        title: None,
        description: None,
        completed: None,
        quantity: Some(quantity),
        actual_quantity: Some(actual_quantity),
        unit: Some(unit_val),
        start_date: None,
        start_time: None,
        deadline: None,
        deadline_time: None,
        hard_deadline: None,
        estimated_duration: None,
    };
    let item = domain::items::update(&pool, &user.id, &item_id, &req)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("item not found".to_string()))?;
    Ok(domain_item_to_shared(item))
}

/// Fetch a single item by id.
#[server(prefix = "/leptos")]
pub async fn get_item(item_id: String) -> Result<Item, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::items::get_one(&pool, &item_id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .map(domain_item_to_shared)
        .ok_or_else(|| ServerFnError::new("item not found".to_string()))
}

/// Update only the description of an item. Pass empty string to clear.
#[server(prefix = "/leptos")]
pub async fn update_item_description(
    item_id: String,
    description: String,
) -> Result<Item, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let req = domain::items::UpdateItemRequest {
        title: None,
        description: Some(if description.trim().is_empty() {
            None
        } else {
            Some(description)
        }),
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
    };
    let item = domain::items::update(&pool, &user.id, &item_id, &req)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("item not found".to_string()))?;
    Ok(domain_item_to_shared(item))
}

/// Update date fields of an item.
/// Pass empty string to clear a date field, None to leave it unchanged.
#[server(prefix = "/leptos")]
pub async fn update_item_dates(
    item_id: String,
    start_date: Option<String>,
    start_time: Option<String>,
    deadline: Option<String>,
    deadline_time: Option<String>,
    hard_deadline: Option<String>,
) -> Result<Item, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;

    let to_field = |s: Option<String>| s.map(empty_as_none);

    let req = domain::items::UpdateItemRequest {
        title: None,
        description: None,
        completed: None,
        quantity: None,
        actual_quantity: None,
        unit: None,
        start_date: to_field(start_date),
        start_time: to_field(start_time),
        deadline: to_field(deadline),
        deadline_time: to_field(deadline_time),
        hard_deadline: to_field(hard_deadline),
        estimated_duration: None,
    };
    let item = domain::items::update(&pool, &user.id, &item_id, &req)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("item not found".to_string()))?;
    Ok(domain_item_to_shared(item))
}

/// Update title and description of an item.
/// Pass `description` as empty string to clear it.
#[server(prefix = "/leptos")]
pub async fn update_item(
    item_id: String,
    title: String,
    description: String,
) -> Result<Item, ServerFnError> {
    if title.trim().is_empty() {
        return Err(ServerFnError::new("title cannot be empty".to_string()));
    }
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let req = domain::items::UpdateItemRequest {
        title: Some(title),
        description: Some(if description.trim().is_empty() {
            None
        } else {
            Some(description)
        }),
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
    };
    let item = domain::items::update(&pool, &user.id, &item_id, &req)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("item not found".to_string()))?;
    Ok(domain_item_to_shared(item))
}

#[cfg(feature = "ssr")]
fn empty_as_none(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}

#[cfg(feature = "ssr")]
fn build_list_names(lists: Vec<domain::lists::List>) -> std::collections::HashMap<String, String> {
    lists.into_iter().map(|l| (l.id, l.name)).collect()
}

#[cfg(feature = "ssr")]
fn items_to_date_items(
    items: Vec<domain::items::Item>,
    list_names: &std::collections::HashMap<String, String>,
) -> Vec<DateItem> {
    items
        .into_iter()
        .map(|item| {
            let list_name = list_names.get(&item.list_id).cloned().unwrap_or_default();
            DateItem {
                item: domain_item_to_shared(item),
                list_name,
            }
        })
        .collect()
}

/// Fetch today's items (start/deadline/hard_deadline = today) and overdue items
/// (incomplete items with deadline before today), both enriched with list names.
#[server(prefix = "/leptos")]
pub async fn get_today_data() -> Result<TodayData, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;

    // Resolve today's date in user's timezone first
    let prefs = domain::preferences::get(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let tz: chrono_tz::Tz = prefs.timezone.parse().unwrap_or(chrono_tz::UTC);
    let today_date = chrono::Utc::now()
        .with_timezone(&tz)
        .format("%Y-%m-%d")
        .to_string();

    // Pass resolved date string (not "today") to avoid double timezone resolution inside domain
    let today_items = domain::items::by_date(&pool, &user.id, &today_date)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let overdue_items = domain::items::overdue(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let all_lists = domain::lists::list_all(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let list_names = build_list_names(all_lists);

    Ok(TodayData {
        today_date,
        today: items_to_date_items(today_items, &list_names),
        overdue: items_to_date_items(overdue_items, &list_names),
    })
}

/// Fetch items for a specific date (any of start_date, deadline, hard_deadline matches),
/// enriched with list names. Pass a "YYYY-MM-DD" string.
#[server(prefix = "/leptos")]
pub async fn get_items_by_date(date: String) -> Result<Vec<DateItem>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;

    let items = domain::items::by_date(&pool, &user.id, &date)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let all_lists = domain::lists::list_all(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let list_names = build_list_names(all_lists);
    Ok(items_to_date_items(items, &list_names))
}

/// Fetch all items for the current user, enriched with list names.
#[server(prefix = "/leptos")]
pub async fn get_all_items() -> Result<Vec<DateItem>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;

    let items = domain::items::list_all_for_user(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let all_lists = domain::lists::list_all(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let list_names = build_list_names(all_lists);
    Ok(items_to_date_items(items, &list_names))
}

/// Fetch items for each day of the week containing `start_date` (Mon–Sun).
/// Pass "YYYY-MM-DD" — the server computes the Monday of that week.
#[server(prefix = "/leptos")]
pub async fn get_calendar_week(start_date: String) -> Result<Vec<CalendarWeekDay>, ServerFnError> {
    use chrono::{Datelike, Duration, NaiveDate};
    use std::str::FromStr;

    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;

    let date = NaiveDate::from_str(&start_date)
        .map_err(|_| ServerFnError::new("invalid date".to_string()))?;
    let monday = date - Duration::days(date.weekday().num_days_from_monday() as i64);

    let week_dates: Vec<NaiveDate> = (0..7).map(|i| monday + Duration::days(i)).collect();

    let all_lists = domain::lists::list_all(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let list_names = build_list_names(all_lists);

    let mut days = Vec::with_capacity(7);
    for d in week_dates {
        let date_str = d.format("%Y-%m-%d").to_string();
        let items = domain::items::by_date(&pool, &user.id, &date_str)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        days.push(CalendarWeekDay {
            date: date_str,
            items: items_to_date_items(items, &list_names),
        });
    }
    Ok(days)
}

/// Fetch calendar month data: grid metadata + per-day item counts.
/// Pass `year_month` as "YYYY-MM"; pass an empty string to get the current month.
#[server(prefix = "/leptos")]
pub async fn get_calendar_month(year_month: String) -> Result<CalendarMonthData, ServerFnError> {
    use chrono::{Datelike, NaiveDate};
    use kartoteka_shared::types::{CalendarDay, FlexDate};

    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;

    // Resolve empty string → current month in user's timezone
    let ym = if year_month.is_empty() {
        let prefs = domain::preferences::get(&pool, &user.id)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        let tz: chrono_tz::Tz = prefs.timezone.parse().unwrap_or(chrono_tz::UTC);
        chrono::Utc::now()
            .with_timezone(&tz)
            .format("%Y-%m")
            .to_string()
    } else {
        year_month
    };

    // Parse "YYYY-MM"
    let (year_str, month_str) = ym
        .split_once('-')
        .ok_or_else(|| ServerFnError::new("year_month must be YYYY-MM".to_string()))?;
    let year: i32 = year_str
        .parse()
        .map_err(|_| ServerFnError::new("invalid year".to_string()))?;
    let month: u32 = month_str
        .parse()
        .map_err(|_| ServerFnError::new("invalid month".to_string()))?;

    let first_day = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| ServerFnError::new("invalid year/month".to_string()))?;
    // ISO weekday: Monday=0, Sunday=6
    let first_weekday = first_day.weekday().num_days_from_monday() as u8;

    let next_month_first = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .ok_or_else(|| ServerFnError::new("date overflow".to_string()))?;
    let days_in_month = next_month_first.signed_duration_since(first_day).num_days() as u8;

    // Fetch items for this month
    let items = domain::items::calendar(&pool, &user.id, &ym)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Compute per-day counts: each item counted once per Day-precision date field
    let mut day_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for item in &items {
        let mut item_dates = std::collections::HashSet::new();
        for date_opt in [&item.start_date, &item.deadline, &item.hard_deadline] {
            if let Some(FlexDate::Day(d)) = date_opt {
                item_dates.insert(d.format("%Y-%m-%d").to_string());
            }
        }
        for date_str in item_dates {
            *day_counts.entry(date_str).or_default() += 1;
        }
    }

    let mut items_by_day: Vec<CalendarDay> = day_counts
        .into_iter()
        .map(|(date, count)| CalendarDay { date, count })
        .collect();
    items_by_day.sort_by(|a, b| a.date.cmp(&b.date));

    Ok(CalendarMonthData {
        year,
        month,
        year_month: ym,
        first_weekday,
        days_in_month,
        items_by_day,
    })
}
