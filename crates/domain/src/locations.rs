use crate::DomainError;
use kartoteka_db::{self as db, locations::InsertLocationInput};
use kartoteka_shared::models::{Country, Location};
use once_cell::sync::Lazy;
use sqlx::SqlitePool;
use std::collections::HashMap;
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

static COUNTRY_ALIASES: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    for c in rust_iso3166::ALL {
        // leak() is safe here: static lifetime, called once
        m.insert(c.name.to_lowercase().leak() as &'static str, c.alpha2);
    }
    let pl_aliases: &[(&str, &str)] = &[
        ("polska", "PL"),
        ("niemcy", "DE"),
        ("francja", "FR"),
        ("włochy", "IT"),
        ("hiszpania", "ES"),
        ("anglia", "GB"),
        ("wielka brytania", "GB"),
        ("stany zjednoczone", "US"),
        ("usa", "US"),
        ("rosja", "RU"),
        ("ukraina", "UA"),
        ("czechy", "CZ"),
        ("słowacja", "SK"),
        ("austria", "AT"),
        ("szwajcaria", "CH"),
        ("holandia", "NL"),
        ("belgia", "BE"),
        ("szwecja", "SE"),
        ("norwegia", "NO"),
        ("dania", "DK"),
        ("finlandia", "FI"),
        ("węgry", "HU"),
        ("rumunia", "RO"),
        ("bułgaria", "BG"),
        ("grecja", "GR"),
        ("portugalia", "PT"),
        ("turcja", "TR"),
        ("japonia", "JP"),
        ("chiny", "CN"),
        ("korea", "KR"),
        ("indie", "IN"),
        ("brazylia", "BR"),
        ("meksyk", "MX"),
        ("kanada", "CA"),
        ("australia", "AU"),
        ("nowa zelandia", "NZ"),
    ];
    for (alias, code) in pl_aliases {
        m.insert(alias, code);
    }
    m
});

fn resolve_country_code(input: &str) -> Option<&'static str> {
    COUNTRY_ALIASES.get(input.to_lowercase().as_str()).copied()
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

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid format: expected 1–4 comma-separated segments")]
    InvalidFormat,
    #[error("unknown country '{0}' and no default set")]
    UnknownCountry(String),
    #[error("ambiguous city")]
    Ambiguous(Vec<Location>),
    #[error(transparent)]
    Db(#[from] kartoteka_db::DbError),
    #[error(transparent)]
    Domain(#[from] DomainError),
}

#[tracing::instrument(fields(action = "parse_location"), skip(pool))]
pub async fn parse_and_resolve(
    pool: &SqlitePool,
    user_id: &str,
    input: &str,
) -> Result<Location, ParseError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ParseError::InvalidFormat);
    }

    // Alias shortcut: single segment with no comma
    if !input.contains(',') {
        if let Some(loc) = db::locations::find_by_alias(pool, user_id, input).await? {
            return Ok(row_to_location(loc));
        }
    }

    let segments: Vec<&str> = input
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if segments.is_empty() || segments.len() > 4 {
        return Err(ParseError::InvalidFormat);
    }

    // Resolve country: last segment if it's a known country name, else use default_country_iso
    let (country_iso, remaining) = {
        let last = *segments.last().unwrap();
        if let Some(iso) = resolve_country_code(last) {
            (iso.to_string(), &segments[..segments.len() - 1])
        } else {
            // Query default_country_iso from user_settings — value is stored as JSON string
            let default: Option<(String,)> = sqlx::query_as(
                "SELECT value FROM user_settings WHERE user_id = ? AND key = 'default_country_iso'",
            )
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(kartoteka_db::DbError::Sqlx)?;
            match default {
                Some((raw,)) => {
                    // Value is stored as JSON string like '"PL"', deserialize it
                    let iso: String = serde_json::from_str(&raw).unwrap_or(raw);
                    (iso, segments.as_slice())
                }
                None => return Err(ParseError::UnknownCountry(last.to_string())),
            }
        }
    };

    let country = db::locations::get_country_by_iso(pool, &country_iso)
        .await?
        .ok_or_else(|| ParseError::UnknownCountry(country_iso.clone()))?;

    if remaining.is_empty() {
        return Err(ParseError::InvalidFormat);
    }

    // Parse city, region, address from remaining segments
    // Format (remaining after country stripped): [address,] city [, region]
    let (city_name, region, address_part) = match remaining.len() {
        1 => (remaining[0], None, None),
        2 => (remaining[1], None, Some(remaining[0])),
        3 => (remaining[1], Some(remaining[2]), Some(remaining[0])),
        _ => return Err(ParseError::InvalidFormat),
    };

    // Resolve city: find existing or create
    let city = {
        let candidates =
            db::locations::find_city(pool, user_id, &country.id, city_name, region).await?;
        match candidates.len() {
            0 => {
                db::locations::insert_location(
                    pool,
                    &InsertLocationInput {
                        id: Uuid::new_v4().to_string(),
                        user_id: user_id.to_string(),
                        name: city_name.to_string(),
                        alias: None,
                        region: region.map(str::to_string),
                        address: None,
                        location_type: "city".to_string(),
                        country_id: country.id.clone(),
                        parent_id: None,
                    },
                )
                .await?
            }
            1 => candidates.into_iter().next().unwrap(),
            _ => {
                return Err(ParseError::Ambiguous(
                    candidates.into_iter().map(row_to_location).collect(),
                ));
            }
        }
    };

    let Some(addr) = address_part else {
        return Ok(row_to_location(city));
    };

    // Resolve address: find existing under this city or create
    let existing_addr: Option<kartoteka_db::types::LocationRow> = sqlx::query_as(
        "SELECT id, user_id, name, alias, region, address, type AS \"type\", country_id, parent_id, lat, lon, created_at \
         FROM locations WHERE user_id = ? AND parent_id = ? AND LOWER(name) = LOWER(?)",
    )
    .bind(user_id)
    .bind(&city.id)
    .bind(addr)
    .fetch_optional(pool)
    .await
    .map_err(kartoteka_db::DbError::Sqlx)?;

    let address_loc = match existing_addr {
        Some(row) => row,
        None => {
            db::locations::insert_location(
                pool,
                &InsertLocationInput {
                    id: Uuid::new_v4().to_string(),
                    user_id: user_id.to_string(),
                    name: addr.to_string(),
                    alias: None,
                    region: None,
                    address: Some(addr.to_string()),
                    location_type: "address".to_string(),
                    country_id: country.id.clone(),
                    parent_id: Some(city.id.clone()),
                },
            )
            .await?
        }
    };

    Ok(row_to_location(address_loc))
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

    #[tokio::test]
    async fn parse_city_only_uses_default_country() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        sqlx::query("INSERT INTO user_settings (user_id, key, value) VALUES (?, 'default_country_iso', '\"PL\"')")
            .bind(&uid)
            .execute(&pool)
            .await
            .unwrap();

        let loc = parse_and_resolve(&pool, &uid, "Olsztyn").await.unwrap();
        assert_eq!(loc.name, "Olsztyn");
        assert_eq!(loc.location_type, "city");
    }

    #[tokio::test]
    async fn parse_address_and_city() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        sqlx::query("INSERT INTO user_settings (user_id, key, value) VALUES (?, 'default_country_iso', '\"PL\"')")
            .bind(&uid)
            .execute(&pool)
            .await
            .unwrap();

        let loc = parse_and_resolve(&pool, &uid, "Kowalska 1, Olsztyn")
            .await
            .unwrap();
        assert_eq!(loc.location_type, "address");
        assert_eq!(loc.name, "Kowalska 1");
    }

    #[tokio::test]
    async fn parse_with_explicit_country() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let loc = parse_and_resolve(&pool, &uid, "Berlin, Niemcy")
            .await
            .unwrap();
        assert_eq!(loc.name, "Berlin");
        let de = db::locations::get_country_by_iso(&pool, "DE")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loc.country_id, de.id);
    }

    #[tokio::test]
    async fn parse_reuses_existing_city() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        sqlx::query("INSERT INTO user_settings (user_id, key, value) VALUES (?, 'default_country_iso', '\"PL\"')")
            .bind(&uid)
            .execute(&pool)
            .await
            .unwrap();

        let first = parse_and_resolve(&pool, &uid, "Kraków").await.unwrap();
        let second = parse_and_resolve(&pool, &uid, "kraków").await.unwrap();
        assert_eq!(first.id, second.id);
    }

    #[tokio::test]
    async fn parse_alias_resolves_to_location() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let de = db::locations::get_country_by_iso(&pool, "DE")
            .await
            .unwrap()
            .unwrap();
        let city = create_location(
            &pool,
            &uid,
            &CreateLocationInput {
                name: "Berlin".to_string(),
                alias: Some("Baza".to_string()),
                region: None,
                address: None,
                location_type: "city".to_string(),
                country_id: de.id.clone(),
                parent_id: None,
            },
        )
        .await
        .unwrap();

        let resolved = parse_and_resolve(&pool, &uid, "Baza").await.unwrap();
        assert_eq!(resolved.id, city.id);
    }

    #[tokio::test]
    async fn parse_ambiguous_returns_error() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        sqlx::query("INSERT INTO user_settings (user_id, key, value) VALUES (?, 'default_country_iso', '\"PL\"')")
            .bind(&uid)
            .execute(&pool)
            .await
            .unwrap();
        let pl = db::locations::get_country_by_iso(&pool, "PL")
            .await
            .unwrap()
            .unwrap();

        create_location(
            &pool,
            &uid,
            &CreateLocationInput {
                name: "Nowe Miasto".to_string(),
                alias: None,
                region: Some("mazowieckie".to_string()),
                address: None,
                location_type: "city".to_string(),
                country_id: pl.id.clone(),
                parent_id: None,
            },
        )
        .await
        .unwrap();
        create_location(
            &pool,
            &uid,
            &CreateLocationInput {
                name: "Nowe Miasto".to_string(),
                alias: None,
                region: Some("podlaskie".to_string()),
                address: None,
                location_type: "city".to_string(),
                country_id: pl.id.clone(),
                parent_id: None,
            },
        )
        .await
        .unwrap();

        let result = parse_and_resolve(&pool, &uid, "Nowe Miasto").await;
        assert!(matches!(result, Err(ParseError::Ambiguous(_))));
    }

    #[tokio::test]
    async fn parse_too_many_segments_error() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let result = parse_and_resolve(&pool, &uid, "a, b, c, d, e").await;
        assert!(matches!(result, Err(ParseError::InvalidFormat)));
    }
}
