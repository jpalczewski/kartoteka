use crate::{DbError, SqlitePool};
use chrono::{DateTime, Utc};
use sqlx::FromRow;

pub mod clients {
    use super::*;

    #[derive(Debug, Clone, FromRow)]
    pub struct OAuthClient {
        pub client_id: String,
        pub name: String,
        pub redirect_uris: String,
        pub created_at: DateTime<Utc>,
        pub last_used_at: Option<DateTime<Utc>>,
    }

    pub async fn create(
        pool: &SqlitePool,
        client_id: &str,
        name: &str,
        redirect_uris_json: &str,
    ) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO oauth_clients (client_id, name, redirect_uris, created_at) \
             VALUES (?, ?, ?, datetime('now'))",
        )
        .bind(client_id)
        .bind(name)
        .bind(redirect_uris_json)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
        Ok(())
    }

    pub async fn find(pool: &SqlitePool, client_id: &str) -> Result<Option<OAuthClient>, DbError> {
        sqlx::query_as(
            "SELECT client_id, name, redirect_uris, created_at, last_used_at \
             FROM oauth_clients WHERE client_id = ?",
        )
        .bind(client_id)
        .fetch_optional(pool)
        .await
        .map_err(DbError::Sqlx)
    }
}

pub mod codes {
    use super::*;

    #[derive(Debug, Clone, FromRow)]
    pub struct AuthCode {
        pub code: String,
        pub client_id: String,
        pub user_id: String,
        pub code_challenge: String,
        pub scope: String,
        pub redirect_uri: String,
        pub expires_at: DateTime<Utc>,
        pub used: i64,
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert(
        pool: &SqlitePool,
        code: &str,
        client_id: &str,
        user_id: &str,
        code_challenge: &str,
        scope: &str,
        redirect_uri: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO oauth_authorization_codes \
             (code, client_id, user_id, code_challenge, scope, redirect_uri, expires_at, used) \
             VALUES (?, ?, ?, ?, ?, ?, ?, 0)",
        )
        .bind(code)
        .bind(client_id)
        .bind(user_id)
        .bind(code_challenge)
        .bind(scope)
        .bind(redirect_uri)
        .bind(expires_at)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
        Ok(())
    }

    pub async fn consume(
        pool: &SqlitePool,
        code: &str,
        client_id: &str,
    ) -> Result<Option<AuthCode>, DbError> {
        sqlx::query_as(
            "UPDATE oauth_authorization_codes SET used = 1 \
             WHERE code = ? AND client_id = ? AND used = 0 AND expires_at > datetime('now') \
             RETURNING code, client_id, user_id, code_challenge, scope, redirect_uri, expires_at, used",
        )
        .bind(code)
        .bind(client_id)
        .fetch_optional(pool)
        .await
        .map_err(DbError::Sqlx)
    }
}

pub mod refresh {
    use super::*;

    #[derive(Debug, Clone, FromRow)]
    pub struct RefreshToken {
        pub token_hash: String,
        pub client_id: String,
        pub user_id: String,
        pub scope: String,
        pub expires_at: DateTime<Utc>,
        pub created_at: DateTime<Utc>,
    }

    pub async fn insert(
        pool: &SqlitePool,
        token_hash: &str,
        client_id: &str,
        user_id: &str,
        scope: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO oauth_refresh_tokens \
             (token_hash, client_id, user_id, scope, expires_at, created_at) \
             VALUES (?, ?, ?, ?, ?, datetime('now'))",
        )
        .bind(token_hash)
        .bind(client_id)
        .bind(user_id)
        .bind(scope)
        .bind(expires_at)
        .execute(pool)
        .await
        .map_err(DbError::Sqlx)?;
        Ok(())
    }

    pub async fn find_and_delete(
        pool: &SqlitePool,
        token_hash: &str,
    ) -> Result<Option<RefreshToken>, DbError> {
        sqlx::query_as(
            "DELETE FROM oauth_refresh_tokens \
             WHERE token_hash = ? AND expires_at > datetime('now') \
             RETURNING token_hash, client_id, user_id, scope, expires_at, created_at",
        )
        .bind(token_hash)
        .fetch_optional(pool)
        .await
        .map_err(DbError::Sqlx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_user, test_pool};
    use chrono::Duration;

    #[tokio::test]
    async fn client_create_and_find() {
        let pool = test_pool().await;
        clients::create(&pool, "cli-1", "Claude", r#"["http://x/cb"]"#)
            .await
            .unwrap();
        let c = clients::find(&pool, "cli-1").await.unwrap().unwrap();
        assert_eq!(c.name, "Claude");
        assert_eq!(c.redirect_uris, r#"["http://x/cb"]"#);
        assert!(clients::find(&pool, "no").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn code_single_use_and_expiry() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        clients::create(&pool, "c1", "X", r#"["http://x"]"#)
            .await
            .unwrap();
        let future = Utc::now() + Duration::minutes(5);
        codes::insert(
            &pool, "code-a", "c1", &uid, "chal", "mcp", "http://x", future,
        )
        .await
        .unwrap();

        let got = codes::consume(&pool, "code-a", "c1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.user_id, uid);
        assert!(
            codes::consume(&pool, "code-a", "c1")
                .await
                .unwrap()
                .is_none()
        );
        codes::insert(
            &pool, "code-b", "c1", &uid, "chal", "mcp", "http://x", future,
        )
        .await
        .unwrap();
        assert!(
            codes::consume(&pool, "code-b", "other")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn refresh_rotation_deletes_old() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        clients::create(&pool, "c1", "X", r#"["http://x"]"#)
            .await
            .unwrap();
        let future = Utc::now() + Duration::days(30);
        refresh::insert(&pool, "hash-1", "c1", &uid, "mcp", future)
            .await
            .unwrap();
        let t = refresh::find_and_delete(&pool, "hash-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(t.user_id, uid);
        assert!(
            refresh::find_and_delete(&pool, "hash-1")
                .await
                .unwrap()
                .is_none()
        );
    }
}
