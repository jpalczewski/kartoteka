use kartoteka_shared::models::{Country, Location};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    axum_login::AuthSession, kartoteka_auth::KartotekaBackend, kartoteka_domain as domain,
    sqlx::SqlitePool,
};

#[server(prefix = "/leptos")]
pub async fn get_location_detail_sf(id: String) -> Result<Option<Location>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::locations::get_location(&pool, &user.id, &id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(prefix = "/leptos")]
pub async fn get_city_addresses_sf(city_id: String) -> Result<Vec<Location>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let all = domain::locations::list_locations(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(all
        .into_iter()
        .filter(|l| l.parent_id.as_deref() == Some(city_id.as_str()))
        .collect())
}

#[server(prefix = "/leptos")]
pub async fn get_countries() -> Result<Vec<Country>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    domain::locations::list_countries(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(prefix = "/leptos")]
pub async fn get_locations_sf() -> Result<Vec<Location>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::locations::list_locations(&pool, &user.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(prefix = "/leptos")]
pub async fn create_location_sf(
    name: String,
    alias: Option<String>,
    region: Option<String>,
    address: Option<String>,
    location_type: String,
    country_id: String,
    parent_id: Option<String>,
) -> Result<Location, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::locations::create_location(
        &pool,
        &user.id,
        &domain::locations::CreateLocationInput {
            name,
            alias,
            region,
            address,
            location_type,
            country_id,
            parent_id,
        },
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(prefix = "/leptos")]
pub async fn update_location_sf(
    id: String,
    name: Option<String>,
    alias: Option<String>,
    clear_alias: bool,
    region: Option<String>,
    address: Option<String>,
    clear_address: bool,
) -> Result<Option<Location>, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    let alias_field = if clear_alias {
        Some(None)
    } else {
        alias.map(Some)
    };
    let address_field = if clear_address {
        Some(None)
    } else {
        address.map(Some)
    };
    domain::locations::update_location(
        &pool,
        &user.id,
        &id,
        &domain::locations::UpdateLocationInput {
            name,
            alias: alias_field,
            region,
            address: address_field,
        },
    )
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(prefix = "/leptos")]
pub async fn delete_location_sf(id: String) -> Result<bool, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::locations::delete_location(&pool, &user.id, &id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server(prefix = "/leptos")]
pub async fn parse_location_sf(input: String) -> Result<Location, ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    domain::locations::parse_and_resolve(&pool, &user.id, &input)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
