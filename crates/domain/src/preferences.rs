use crate::DomainError;
use kartoteka_db as db;
use kartoteka_shared::Locale;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub timezone: String,
    pub locale: Locale,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePreferencesRequest {
    pub timezone: Option<String>,
    pub locale: Option<Locale>,
}

#[tracing::instrument(skip(pool))]
pub async fn get(pool: &SqlitePool, user_id: &str) -> Result<Preferences, DomainError> {
    let timezone = db::preferences::get_timezone(pool, user_id).await?;
    let locale_str = db::preferences::get_locale(pool, user_id).await?;
    let locale = locale_str.parse().unwrap_or_default();
    Ok(Preferences { timezone, locale })
}

#[tracing::instrument(skip(pool))]
pub async fn update(
    pool: &SqlitePool,
    user_id: &str,
    req: &UpdatePreferencesRequest,
) -> Result<Preferences, DomainError> {
    if let Some(ref tz) = req.timezone {
        db::preferences::set_timezone(pool, user_id, tz).await?;
    }
    if let Some(locale) = req.locale {
        db::preferences::set_locale(pool, user_id, locale.as_str()).await?;
    }
    get(pool, user_id).await
}

pub async fn get_locale(pool: &SqlitePool, user_id: &str) -> Result<Locale, DomainError> {
    let s = db::preferences::get_locale(pool, user_id).await?;
    Ok(s.parse().unwrap_or_default())
}

pub async fn set_locale(
    pool: &SqlitePool,
    user_id: &str,
    locale: Locale,
) -> Result<(), DomainError> {
    db::preferences::set_locale(pool, user_id, locale.as_str())
        .await
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{create_test_user, test_pool};

    #[tokio::test]
    async fn get_defaults() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let prefs = get(&pool, &uid).await.unwrap();
        assert_eq!(prefs.timezone, "UTC");
        assert_eq!(prefs.locale, Locale::En);
    }

    #[tokio::test]
    async fn update_timezone() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let prefs = update(
            &pool,
            &uid,
            &UpdatePreferencesRequest {
                timezone: Some("America/New_York".to_string()),
                locale: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(prefs.timezone, "America/New_York");
        assert_eq!(prefs.locale, Locale::En);
    }

    #[tokio::test]
    async fn update_locale() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let prefs = update(
            &pool,
            &uid,
            &UpdatePreferencesRequest {
                timezone: None,
                locale: Some(Locale::Pl),
            },
        )
        .await
        .unwrap();

        assert_eq!(prefs.timezone, "UTC");
        assert_eq!(prefs.locale, Locale::Pl);
    }

    #[tokio::test]
    async fn update_both() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        let prefs = update(
            &pool,
            &uid,
            &UpdatePreferencesRequest {
                timezone: Some("Europe/Warsaw".to_string()),
                locale: Some(Locale::Pl),
            },
        )
        .await
        .unwrap();

        assert_eq!(prefs.timezone, "Europe/Warsaw");
        assert_eq!(prefs.locale, Locale::Pl);
    }

    #[tokio::test]
    async fn update_empty_request_preserves_values() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;

        update(
            &pool,
            &uid,
            &UpdatePreferencesRequest {
                timezone: Some("Asia/Tokyo".to_string()),
                locale: Some(Locale::Pl),
            },
        )
        .await
        .unwrap();

        let prefs = update(
            &pool,
            &uid,
            &UpdatePreferencesRequest {
                timezone: None,
                locale: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(prefs.timezone, "Asia/Tokyo");
        assert_eq!(prefs.locale, Locale::Pl);
    }
}
