use crate::DomainError;
use kartoteka_db::{self as db, locations::InsertLocationInput};
use kartoteka_shared::models::{Country, Location};
use sqlx::SqlitePool;
use uuid::Uuid;

fn row_to_country(r: db::types::CountryRow) -> Country {
    Country {
        id: r.id,
        iso_code: r.iso_code,
    }
}

fn row_to_location(r: db::types::LocationRow) -> Location {
    Location {
        id: r.id,
        user_id: r.user_id,
        name: r.name,
        alias: r.alias,
        region: r.region,
        address: r.address,
        location_type: r.r#type,
        country_id: r.country_id,
        parent_id: r.parent_id,
        lat: r.lat,
        lon: r.lon,
        created_at: r.created_at,
    }
}

#[derive(Debug)]
pub struct CreateLocationInput {
    pub name: String,
    pub alias: Option<String>,
    pub region: Option<String>,
    pub address: Option<String>,
    pub location_type: String,
    pub country_id: String,
    pub parent_id: Option<String>,
}

#[derive(Debug)]
pub struct UpdateLocationInput {
    pub name: Option<String>,
    pub alias: Option<Option<String>>,
    pub region: Option<String>,
    pub address: Option<Option<String>>,
}

pub async fn list_countries(pool: &SqlitePool) -> Result<Vec<Country>, DomainError> {
    Ok(db::locations::list_countries(pool)
        .await?
        .into_iter()
        .map(row_to_country)
        .collect())
}

pub async fn list_locations(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<Location>, DomainError> {
    Ok(db::locations::list_locations(pool, user_id)
        .await?
        .into_iter()
        .map(row_to_location)
        .collect())
}

pub async fn create_location(
    pool: &SqlitePool,
    user_id: &str,
    input: &CreateLocationInput,
) -> Result<Location, DomainError> {
    validate_location_type(&input.location_type)?;
    if input.location_type == "address" && input.parent_id.is_none() {
        return Err(DomainError::Validation("address_requires_parent_city"));
    }
    let row = db::locations::insert_location(
        pool,
        &InsertLocationInput {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            name: input.name.clone(),
            alias: input.alias.clone(),
            region: input.region.clone(),
            address: input.address.clone(),
            location_type: input.location_type.clone(),
            country_id: input.country_id.clone(),
            parent_id: input.parent_id.clone(),
        },
    )
    .await?;
    Ok(row_to_location(row))
}

pub async fn update_location(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
    input: &UpdateLocationInput,
) -> Result<Option<Location>, DomainError> {
    Ok(db::locations::update_location(
        pool,
        &db::locations::UpdateLocationInput {
            id: id.to_string(),
            user_id: user_id.to_string(),
            name: input.name.clone(),
            alias: input.alias.clone(),
            region: input.region.clone(),
            address: input.address.clone(),
        },
    )
    .await?
    .map(row_to_location))
}

pub async fn delete_location(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
) -> Result<bool, DomainError> {
    Ok(db::locations::delete_location(pool, id, user_id).await?)
}

fn validate_location_type(t: &str) -> Result<(), DomainError> {
    if !["city", "address"].contains(&t) {
        return Err(DomainError::Validation("invalid_location_type"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{create_test_user, test_pool};

    async fn pl_country_id(pool: &SqlitePool) -> String {
        db::locations::get_country_by_iso(pool, "PL")
            .await
            .unwrap()
            .unwrap()
            .id
    }

    #[tokio::test]
    async fn list_countries_returns_all() {
        let pool = test_pool().await;
        let countries = list_countries(&pool).await.unwrap();
        assert!(countries.len() > 200);
    }

    #[tokio::test]
    async fn create_and_list_city() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let country_id = pl_country_id(&pool).await;

        let loc = create_location(
            &pool,
            &uid,
            &CreateLocationInput {
                name: "Warszawa".to_string(),
                alias: None,
                region: Some("mazowieckie".to_string()),
                address: None,
                location_type: "city".to_string(),
                country_id,
                parent_id: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(loc.name, "Warszawa");

        let all = list_locations(&pool, &uid).await.unwrap();
        assert_eq!(all.len(), 1);
    }

    #[tokio::test]
    async fn delete_city_removes_it() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let country_id = pl_country_id(&pool).await;

        let loc = create_location(
            &pool,
            &uid,
            &CreateLocationInput {
                name: "Gdańsk".to_string(),
                alias: None,
                region: None,
                address: None,
                location_type: "city".to_string(),
                country_id,
                parent_id: None,
            },
        )
        .await
        .unwrap();

        assert!(delete_location(&pool, &uid, &loc.id).await.unwrap());
        let all = list_locations(&pool, &uid).await.unwrap();
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn update_alias() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let country_id = pl_country_id(&pool).await;

        let loc = create_location(
            &pool,
            &uid,
            &CreateLocationInput {
                name: "Olsztyn".to_string(),
                alias: None,
                region: None,
                address: None,
                location_type: "city".to_string(),
                country_id,
                parent_id: None,
            },
        )
        .await
        .unwrap();

        let updated = update_location(
            &pool,
            &uid,
            &loc.id,
            &UpdateLocationInput {
                name: None,
                alias: Some(Some("Moje miasto".to_string())),
                region: None,
                address: None,
            },
        )
        .await
        .unwrap()
        .unwrap();

        assert_eq!(updated.alias, Some("Moje miasto".to_string()));
    }
}
