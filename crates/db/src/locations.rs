use crate::{
    DbError,
    types::{CountryRow, LocationRow},
};
use sqlx::SqlitePool;

pub async fn list_countries(pool: &SqlitePool) -> Result<Vec<CountryRow>, DbError> {
    Ok(
        sqlx::query_as::<_, CountryRow>("SELECT id, iso_code FROM countries ORDER BY iso_code")
            .fetch_all(pool)
            .await?,
    )
}

pub async fn get_country_by_iso(
    pool: &SqlitePool,
    iso_code: &str,
) -> Result<Option<CountryRow>, DbError> {
    Ok(sqlx::query_as::<_, CountryRow>(
        "SELECT id, iso_code FROM countries WHERE UPPER(iso_code) = UPPER(?)",
    )
    .bind(iso_code)
    .fetch_optional(pool)
    .await?)
}

pub async fn insert_location(
    pool: &SqlitePool,
    input: &InsertLocationInput,
) -> Result<LocationRow, DbError> {
    sqlx::query(
        "INSERT INTO locations (id, user_id, name, alias, region, address, type, country_id, parent_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&input.id)
    .bind(&input.user_id)
    .bind(&input.name)
    .bind(&input.alias)
    .bind(&input.region)
    .bind(&input.address)
    .bind(&input.location_type)
    .bind(&input.country_id)
    .bind(&input.parent_id)
    .execute(pool)
    .await?;
    get_location(pool, &input.id, &input.user_id)
        .await?
        .ok_or(DbError::NotFound("location"))
}

pub async fn get_location(
    pool: &SqlitePool,
    id: &str,
    user_id: &str,
) -> Result<Option<LocationRow>, DbError> {
    Ok(sqlx::query_as::<_, LocationRow>(
        "SELECT id, user_id, name, alias, region, address, type AS \"type\", country_id, parent_id, lat, lon, created_at \
         FROM locations WHERE id = ? AND user_id = ?",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?)
}

pub async fn list_locations(pool: &SqlitePool, user_id: &str) -> Result<Vec<LocationRow>, DbError> {
    Ok(sqlx::query_as::<_, LocationRow>(
        "SELECT id, user_id, name, alias, region, address, type AS \"type\", country_id, parent_id, lat, lon, created_at \
         FROM locations WHERE user_id = ? ORDER BY type, name",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?)
}

pub async fn update_location(
    pool: &SqlitePool,
    input: &UpdateLocationInput,
) -> Result<Option<LocationRow>, DbError> {
    let rows = sqlx::query(
        "UPDATE locations SET \
         name    = COALESCE(?, name), \
         alias   = CASE WHEN ? THEN NULL WHEN ? IS NOT NULL THEN ? ELSE alias END, \
         region  = COALESCE(?, region), \
         address = CASE WHEN ? THEN NULL WHEN ? IS NOT NULL THEN ? ELSE address END \
         WHERE id = ? AND user_id = ?",
    )
    .bind(input.name.as_deref())
    .bind(matches!(input.alias, Some(None)))
    .bind(input.alias.as_ref().and_then(|a| a.as_deref()))
    .bind(input.alias.as_ref().and_then(|a| a.as_deref()))
    .bind(input.region.as_deref())
    .bind(matches!(input.address, Some(None)))
    .bind(input.address.as_ref().and_then(|a| a.as_deref()))
    .bind(input.address.as_ref().and_then(|a| a.as_deref()))
    .bind(&input.id)
    .bind(&input.user_id)
    .execute(pool)
    .await?
    .rows_affected();
    if rows == 0 {
        return Ok(None);
    }
    get_location(pool, &input.id, &input.user_id).await
}

pub async fn exists_for_user(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM locations WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .fetch_one(pool)
            .await?;
    Ok(count > 0)
}

pub async fn delete_location(pool: &SqlitePool, id: &str, user_id: &str) -> Result<bool, DbError> {
    let rows = sqlx::query("DELETE FROM locations WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(rows > 0)
}

pub async fn find_city(
    pool: &SqlitePool,
    user_id: &str,
    country_id: &str,
    name: &str,
    region: Option<&str>,
) -> Result<Vec<LocationRow>, DbError> {
    Ok(match region {
        Some(r) => sqlx::query_as::<_, LocationRow>(
            "SELECT id, user_id, name, alias, region, address, type AS \"type\", country_id, parent_id, lat, lon, created_at \
             FROM locations WHERE user_id = ? AND country_id = ? AND type = 'city' \
             AND LOWER(name) = LOWER(?) AND LOWER(COALESCE(region,'')) = LOWER(?)",
        )
        .bind(user_id)
        .bind(country_id)
        .bind(name)
        .bind(r)
        .fetch_all(pool)
        .await?,
        None => sqlx::query_as::<_, LocationRow>(
            "SELECT id, user_id, name, alias, region, address, type AS \"type\", country_id, parent_id, lat, lon, created_at \
             FROM locations WHERE user_id = ? AND country_id = ? AND type = 'city' AND LOWER(name) = LOWER(?)",
        )
        .bind(user_id)
        .bind(country_id)
        .bind(name)
        .fetch_all(pool)
        .await?,
    })
}

pub async fn find_by_alias(
    pool: &SqlitePool,
    user_id: &str,
    alias: &str,
) -> Result<Option<LocationRow>, DbError> {
    Ok(sqlx::query_as::<_, LocationRow>(
        "SELECT id, user_id, name, alias, region, address, type AS \"type\", country_id, parent_id, lat, lon, created_at \
         FROM locations WHERE user_id = ? AND LOWER(alias) = LOWER(?)",
    )
    .bind(user_id)
    .bind(alias)
    .fetch_optional(pool)
    .await?)
}

pub async fn subtree_location_ids(
    pool: &SqlitePool,
    root_id: &str,
) -> Result<Vec<String>, DbError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "WITH subtree AS ( \
             SELECT id FROM locations WHERE id = ? \
             UNION ALL \
             SELECT l.id FROM locations l JOIN subtree s ON l.parent_id = s.id \
         ) SELECT id FROM subtree",
    )
    .bind(root_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

#[derive(Debug)]
pub struct InsertLocationInput {
    pub id: String,
    pub user_id: String,
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
    pub id: String,
    pub user_id: String,
    pub name: Option<String>,
    pub alias: Option<Option<String>>,
    pub region: Option<String>,
    pub address: Option<Option<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_user, test_pool};
    use uuid::Uuid;

    #[tokio::test]
    async fn countries_seeded() {
        let pool = test_pool().await;
        let countries = list_countries(&pool).await.unwrap();
        assert!(countries.len() > 200);
    }

    #[tokio::test]
    async fn get_country_pl() {
        let pool = test_pool().await;
        let c = get_country_by_iso(&pool, "PL").await.unwrap();
        assert!(c.is_some());
        assert_eq!(c.unwrap().iso_code, "PL");
    }

    #[tokio::test]
    async fn insert_and_get_city() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let pl = get_country_by_iso(&pool, "PL").await.unwrap().unwrap();

        let loc = insert_location(
            &pool,
            &InsertLocationInput {
                id: Uuid::new_v4().to_string(),
                user_id: uid.clone(),
                name: "Olsztyn".to_string(),
                alias: None,
                region: Some("warmińsko-mazurskie".to_string()),
                address: None,
                location_type: "city".to_string(),
                country_id: pl.id.clone(),
                parent_id: None,
            },
        )
        .await
        .unwrap();

        let fetched = get_location(&pool, &loc.id, &uid).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "Olsztyn");
    }

    #[tokio::test]
    async fn find_city_case_insensitive() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let pl = get_country_by_iso(&pool, "PL").await.unwrap().unwrap();

        insert_location(
            &pool,
            &InsertLocationInput {
                id: Uuid::new_v4().to_string(),
                user_id: uid.clone(),
                name: "Kraków".to_string(),
                alias: None,
                region: None,
                address: None,
                location_type: "city".to_string(),
                country_id: pl.id.clone(),
                parent_id: None,
            },
        )
        .await
        .unwrap();

        let found = find_city(&pool, &uid, &pl.id, "kraków", None)
            .await
            .unwrap();
        assert_eq!(found.len(), 1);
    }

    #[tokio::test]
    async fn find_by_alias_works() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let pl = get_country_by_iso(&pool, "PL").await.unwrap().unwrap();
        let city = insert_location(
            &pool,
            &InsertLocationInput {
                id: Uuid::new_v4().to_string(),
                user_id: uid.clone(),
                name: "Olsztyn".to_string(),
                alias: None,
                region: None,
                address: None,
                location_type: "city".to_string(),
                country_id: pl.id.clone(),
                parent_id: None,
            },
        )
        .await
        .unwrap();

        insert_location(
            &pool,
            &InsertLocationInput {
                id: Uuid::new_v4().to_string(),
                user_id: uid.clone(),
                name: "Kowalska 1".to_string(),
                alias: Some("Dom".to_string()),
                region: None,
                address: Some("Kowalska 1".to_string()),
                location_type: "address".to_string(),
                country_id: pl.id.clone(),
                parent_id: Some(city.id.clone()),
            },
        )
        .await
        .unwrap();

        let found = find_by_alias(&pool, &uid, "dom").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().alias.unwrap(), "Dom");
    }

    #[tokio::test]
    async fn subtree_returns_city_and_children() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let pl = get_country_by_iso(&pool, "PL").await.unwrap().unwrap();
        let city = insert_location(
            &pool,
            &InsertLocationInput {
                id: Uuid::new_v4().to_string(),
                user_id: uid.clone(),
                name: "Gdańsk".to_string(),
                alias: None,
                region: None,
                address: None,
                location_type: "city".to_string(),
                country_id: pl.id.clone(),
                parent_id: None,
            },
        )
        .await
        .unwrap();
        insert_location(
            &pool,
            &InsertLocationInput {
                id: Uuid::new_v4().to_string(),
                user_id: uid.clone(),
                name: "Długa 1".to_string(),
                alias: None,
                region: None,
                address: Some("Długa 1".to_string()),
                location_type: "address".to_string(),
                country_id: pl.id.clone(),
                parent_id: Some(city.id.clone()),
            },
        )
        .await
        .unwrap();

        let ids = subtree_location_ids(&pool, &city.id).await.unwrap();
        assert_eq!(ids.len(), 2);
    }
}
