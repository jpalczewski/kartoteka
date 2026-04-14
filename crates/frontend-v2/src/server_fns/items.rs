use kartoteka_shared::types::{CalendarMonthData, DateItem, Item, ListData, TodayData};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    crate::server_fns::home::domain_list_to_shared, axum_login::AuthSession,
    kartoteka_auth::KartotekaBackend, kartoteka_domain as domain, sqlx::SqlitePool,
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

    let list = domain::lists::get_one(&pool, &list_id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("list not found".to_string()))?;

    let items = domain::items::list_for_list(&pool, &list_id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let sublists = domain::lists::sublists(&pool, &list_id, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(ListData {
        list: domain_list_to_shared(list),
        items: items.into_iter().map(domain_item_to_shared).collect(),
        sublists: sublists.into_iter().map(domain_list_to_shared).collect(),
    })
}

/// Add a new item to a list.
#[server(prefix = "/leptos")]
pub async fn create_item(list_id: String, title: String) -> Result<Item, ServerFnError> {
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
    let item = domain::items::create(
        &pool,
        &user.id,
        &list_id,
        &domain::items::CreateItemRequest {
            title,
            description: None,
            quantity: None,
            actual_quantity: None,
            unit: None,
            start_date: None,
            start_time: None,
            deadline: None,
            deadline_time: None,
            hard_deadline: None,
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
