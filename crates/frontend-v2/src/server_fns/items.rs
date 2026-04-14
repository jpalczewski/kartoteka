use kartoteka_shared::types::{Item, ListData};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    axum_login::AuthSession,
    kartoteka_auth::KartotekaBackend,
    kartoteka_domain as domain,
    sqlx::SqlitePool,
    crate::server_fns::home::domain_list_to_shared,
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
