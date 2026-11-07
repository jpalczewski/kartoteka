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

#[derive(Clone, Deserialize)]
pub struct LoginCredentials {
    pub email: String,
    pub password: String,
}

impl std::fmt::Debug for LoginCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoginCredentials")
            .field("email", &self.email)
            .field("password", &"[REDACTED]")
            .finish()
    }
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

/// Build session_auth_hash so that axum-login can invalidate sessions on credential
/// change. Password users: first 32 bytes of the argon2 hash (rotates on password
/// reset). Non-password users: the user id bytes — stable, unique per user, and
/// rotates to the password-derived hash the moment a password auth_method is added.
/// Never returns an empty slice; empty would be shared across all credential-less
/// users and effectively disables session invalidation.
fn build_session_hash(credential: &str, user_id: &str) -> Vec<u8> {
    if credential.is_empty() {
        user_id.as_bytes().to_vec()
    } else {
        credential.as_bytes()[..credential.len().min(32)].to_vec()
    }
}

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
        let session_auth_hash = build_session_hash(&credential, &user_row.id);

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
        let credential =
            db::auth_methods::find_by_user_and_provider(&self.pool, &user_row.id, "password")
                .await?
                .and_then(|m| m.credential)
                .unwrap_or_default();

        // 3. Build session_auth_hash
        let session_auth_hash = build_session_hash(&credential, &user_row.id);

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

/// Look up a KartotekaUser by ID. Used by the POST /auth/2fa handler to complete login
/// after TOTP verification, without requiring access to the private backend in AuthSession.
pub async fn get_user_by_id(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Option<KartotekaUser>, BackendError> {
    let user_row = match db::users::find_by_id(pool, user_id).await? {
        Some(row) => row,
        None => return Ok(None),
    };
    let credential = db::auth_methods::find_by_user_and_provider(pool, &user_row.id, "password")
        .await?
        .and_then(|m| m.credential)
        .unwrap_or_default();
    let session_auth_hash = build_session_hash(&credential, &user_row.id);
    Ok(Some(KartotekaUser {
        id: user_row.id,
        email: user_row.email,
        name: user_row.name,
        avatar_url: user_row.avatar_url,
        role: user_row.role,
        session_auth_hash,
    }))
}

pub type AuthSession = axum_login::AuthSession<KartotekaBackend>;

#[cfg(test)]
mod tests;
