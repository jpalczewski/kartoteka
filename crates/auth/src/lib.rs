use async_trait::async_trait;
use axum_login::{AuthUser, AuthnBackend};
use kartoteka_db as db;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KartotekaUser {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: String,
    /// First bytes of password hash — for session invalidation on password change.
    session_auth_hash: Vec<u8>,
}

impl AuthUser for KartotekaUser {
    type Id = String;

    fn id(&self) -> Self::Id {
        self.id.clone()
    }

    fn session_auth_hash(&self) -> &[u8] {
        &self.session_auth_hash
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginCredentials {
    pub email: String,
    pub password: String,
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("database error: {0}")]
    Db(#[from] db::DbError),
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone)]
pub struct KartotekaBackend {
    pool: SqlitePool,
}

impl KartotekaBackend {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn build_session_hash(credential: &str) -> Vec<u8> {
    credential.as_bytes()[..credential.len().min(32)].to_vec()
}

#[async_trait]
impl AuthnBackend for KartotekaBackend {
    type User = KartotekaUser;
    type Credentials = LoginCredentials;
    type Error = BackendError;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        // 1. Find user by email
        let user_row = match db::users::find_by_email(&self.pool, &creds.email).await? {
            Some(row) => row,
            None => return Ok(None),
        };

        // 2. Find password auth method
        let auth_method =
            match db::auth_methods::find_by_user_and_provider(&self.pool, &user_row.id, "password")
                .await?
            {
                Some(m) => m,
                None => return Ok(None),
            };

        let credential = match auth_method.credential {
            Some(c) => c,
            None => return Ok(None),
        };

        // 3. Verify password
        let valid = kartoteka_domain::auth::verify_password(creds.password, credential.clone())
            .await
            .map_err(|e| BackendError::Other(e.to_string()))?;

        if !valid {
            return Ok(None);
        }

        // 4. Build session_auth_hash
        let session_auth_hash = build_session_hash(&credential);

        // 5. Return user
        Ok(Some(KartotekaUser {
            id: user_row.id,
            email: user_row.email,
            name: user_row.name,
            avatar_url: user_row.avatar_url,
            role: user_row.role,
            session_auth_hash,
        }))
    }

    async fn get_user(
        &self,
        user_id: &axum_login::UserId<Self>,
    ) -> Result<Option<Self::User>, Self::Error> {
        // 1. Find user by id
        let user_row = match db::users::find_by_id(&self.pool, user_id).await? {
            Some(row) => row,
            None => return Ok(None),
        };

        // 2. Find password auth method (get credential, default empty)
        let credential = db::auth_methods::find_by_user_and_provider(
            &self.pool,
            &user_row.id,
            "password",
        )
        .await?
        .and_then(|m| m.credential)
        .unwrap_or_default();

        // 3. Build session_auth_hash
        let session_auth_hash = build_session_hash(&credential);

        Ok(Some(KartotekaUser {
            id: user_row.id,
            email: user_row.email,
            name: user_row.name,
            avatar_url: user_row.avatar_url,
            role: user_row.role,
            session_auth_hash,
        }))
    }
}

pub type AuthSession = axum_login::AuthSession<KartotekaBackend>;

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::test_pool;
    use kartoteka_domain::auth::register;

    #[tokio::test]
    async fn authenticate_correct_password() {
        let pool = test_pool().await;
        register(&pool, "user@example.com", "secret123", None)
            .await
            .unwrap();
        let backend = KartotekaBackend::new(pool);
        let user = backend
            .authenticate(LoginCredentials {
                email: "user@example.com".to_string(),
                password: "secret123".to_string(),
            })
            .await
            .unwrap();
        assert!(user.is_some());
        assert_eq!(user.unwrap().email, "user@example.com");
    }

    #[tokio::test]
    async fn authenticate_wrong_password_returns_none() {
        let pool = test_pool().await;
        register(&pool, "user@example.com", "secret123", None)
            .await
            .unwrap();
        let backend = KartotekaBackend::new(pool);
        let user = backend
            .authenticate(LoginCredentials {
                email: "user@example.com".to_string(),
                password: "wrongpass".to_string(),
            })
            .await
            .unwrap();
        assert!(user.is_none());
    }

    #[tokio::test]
    async fn authenticate_unknown_email_returns_none() {
        let pool = test_pool().await;
        let backend = KartotekaBackend::new(pool);
        let user = backend
            .authenticate(LoginCredentials {
                email: "nobody@example.com".to_string(),
                password: "anything".to_string(),
            })
            .await
            .unwrap();
        assert!(user.is_none());
    }

    #[tokio::test]
    async fn get_user_returns_registered_user() {
        let pool = test_pool().await;
        let info = register(&pool, "user@example.com", "pass", Some("Alice"))
            .await
            .unwrap();
        let backend = KartotekaBackend::new(pool);
        let user = backend.get_user(&info.id).await.unwrap().unwrap();
        assert_eq!(user.name.as_deref(), Some("Alice"));
        assert_eq!(user.role, "admin");
    }

    #[tokio::test]
    async fn get_user_unknown_id_returns_none() {
        let pool = test_pool().await;
        let backend = KartotekaBackend::new(pool);
        assert!(backend
            .get_user(&"no-such-id".to_string())
            .await
            .unwrap()
            .is_none());
    }
}
