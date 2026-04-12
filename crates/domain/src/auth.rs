use crate::DomainError;
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{rand_core::OsRng, SaltString},
};
use kartoteka_db as db;
use serde::Serialize;
use sqlx::SqlitePool;
use totp_rs::{Algorithm, Secret, TOTP};

/// TOTP setup result — secret for manual entry, URL for QR code.
#[derive(Debug, Clone, Serialize)]
pub struct TotpSetup {
    pub secret: String,
    pub otpauth_url: String,
}

/// Minimal user info returned from register.
#[derive(Debug, Clone, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub role: String,
}

/// Hash a password using argon2id. Runs in a blocking thread to avoid blocking async executor.
#[tracing::instrument(skip(password))]
pub async fn hash_password(password: String) -> Result<String, DomainError> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| DomainError::Internal(format!("argon2 hash: {e}")))
    })
    .await
    .map_err(|e| DomainError::Internal(format!("spawn_blocking: {e}")))?
}

/// Verify a password against an argon2 hash. Runs in a blocking thread.
#[tracing::instrument(skip(password, hash))]
pub async fn verify_password(password: String, hash: String) -> Result<bool, DomainError> {
    tokio::task::spawn_blocking(move || {
        let parsed = PasswordHash::new(&hash)
            .map_err(|e| DomainError::Internal(format!("invalid hash: {e}")))?;
        Ok::<bool, DomainError>(
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok(),
        )
    })
    .await
    .map_err(|e| DomainError::Internal(format!("spawn_blocking: {e}")))?
}

fn make_totp(secret_b32: &str) -> Result<TOTP, DomainError> {
    let bytes = Secret::Encoded(secret_b32.to_string())
        .to_bytes()
        .map_err(|e| DomainError::Internal(format!("totp secret decode: {e}")))?;
    TOTP::new(Algorithm::SHA1, 6, 1, 30, bytes, None, String::new())
        .map_err(|e| DomainError::Internal(format!("totp init: {e}")))
}

/// Update a server configuration key.
#[tracing::instrument(skip(pool))]
pub async fn set_server_config(pool: &SqlitePool, key: &str, value: &str) -> Result<(), DomainError> {
    kartoteka_db::server_config::set(pool, key, value).await?;
    Ok(())
}

/// Check whether new registrations are currently allowed.
/// Reads `registration_enabled` from server_config; defaults to true if key is missing.
#[tracing::instrument(skip(pool))]
pub async fn is_registration_enabled(pool: &SqlitePool) -> Result<bool, DomainError> {
    let val = db::server_config::get(pool, "registration_enabled").await?;
    Ok(val.as_deref() != Some("false"))
}

/// Register a new user.
///
/// Rules:
/// - Registration must be enabled
/// - Email must not already be taken
/// - First registered user gets role = "admin"; all subsequent get "user"
/// - Password is hashed with argon2 in spawn_blocking
/// - User row + auth_method row inserted in a transaction
#[tracing::instrument(skip(pool, password))]
pub async fn register(
    pool: &SqlitePool,
    email: &str,
    password: &str,
    name: Option<&str>,
) -> Result<UserInfo, DomainError> {
    // 1. Check registration gate
    if !is_registration_enabled(pool).await? {
        return Err(DomainError::Forbidden);
    }

    // 2. Check email uniqueness
    if db::users::find_by_email(pool, email).await?.is_some() {
        return Err(DomainError::Validation("email_taken"));
    }

    // 3. Determine role: first user is admin
    let user_count = db::users::count(pool).await?;
    let role = if user_count == 0 { "admin" } else { "user" };

    // 4. Hash password in blocking thread
    let hash = hash_password(password.to_string()).await?;

    // 5. Insert user + auth_method in a transaction
    let user_id = uuid::Uuid::new_v4().to_string();
    let method_id = uuid::Uuid::new_v4().to_string();

    let mut tx = pool.begin().await.map_err(db::DbError::Sqlx)?;

    let user = sqlx::query_as::<_, db::types::UserRow>(
        "INSERT INTO users (id, email, name, role) VALUES (?, ?, ?, ?) \
         RETURNING id, email, name, avatar_url, role, created_at, updated_at",
    )
    .bind(&user_id)
    .bind(email)
    .bind(name)
    .bind(role)
    .fetch_one(&mut *tx)
    .await
    .map_err(db::DbError::Sqlx)?;

    sqlx::query(
        "INSERT INTO auth_methods (id, user_id, provider, provider_id, credential) \
         VALUES (?, ?, 'password', ?, ?)",
    )
    .bind(&method_id)
    .bind(&user_id)
    .bind(email)
    .bind(&hash)
    .execute(&mut *tx)
    .await
    .map_err(db::DbError::Sqlx)?;

    tx.commit().await.map_err(db::DbError::Sqlx)?;

    Ok(UserInfo {
        id: user.id,
        email: user.email,
        name: user.name,
        role: user.role,
    })
}

/// Generate a new TOTP secret for the user and store it unverified.
/// Returns base32 secret and otpauth:// URL for QR display.
/// Calling again resets the secret and requires re-verification.
#[tracing::instrument(skip(pool))]
pub async fn setup_totp(
    pool: &SqlitePool,
    user_id: &str,
    email: &str,
) -> Result<TotpSetup, DomainError> {
    let secret = Secret::generate_secret();
    let bytes = secret
        .to_bytes()
        .map_err(|e| DomainError::Internal(format!("totp generate: {e}")))?;
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        bytes,
        Some("Kartoteka".to_string()),
        email.to_string(),
    )
    .map_err(|e| DomainError::Internal(format!("totp init: {e}")))?;
    let secret_b32 = totp.get_secret_base32();
    let url = totp.get_url();
    db::totp::upsert(pool, user_id, &secret_b32).await?;
    Ok(TotpSetup { secret: secret_b32, otpauth_url: url })
}

/// Verify a TOTP code during initial setup. Marks the secret as verified on success.
#[tracing::instrument(skip(pool, code))]
pub async fn verify_totp_setup(
    pool: &SqlitePool,
    user_id: &str,
    code: &str,
) -> Result<(), DomainError> {
    let row = db::totp::find(pool, user_id)
        .await?
        .ok_or(DomainError::NotFound("totp_secret"))?;
    let totp = make_totp(&row.secret)?;
    let valid = totp
        .check_current(code)
        .map_err(|e| DomainError::Internal(format!("system time: {e}")))?;
    if !valid {
        return Err(DomainError::Validation("invalid_totp_code"));
    }
    db::totp::mark_verified(pool, user_id).await?;
    Ok(())
}

/// Disable TOTP for a user by deleting their secret.
#[tracing::instrument(skip(pool))]
pub async fn disable_totp(pool: &SqlitePool, user_id: &str) -> Result<(), DomainError> {
    db::totp::delete(pool, user_id).await?;
    Ok(())
}

/// Verify a TOTP code during login. Returns false if TOTP not enabled or code wrong.
#[tracing::instrument(skip(pool, code))]
pub async fn check_totp_code(
    pool: &SqlitePool,
    user_id: &str,
    code: &str,
) -> Result<bool, DomainError> {
    let row = match db::totp::find(pool, user_id).await? {
        Some(r) if r.verified => r,
        _ => return Ok(false),
    };
    let totp = make_totp(&row.secret)?;
    totp
        .check_current(code)
        .map_err(|e| DomainError::Internal(format!("system time: {e}")))
}

/// Returns true iff the user has a verified TOTP secret.
#[tracing::instrument(skip(pool))]
pub async fn is_totp_enabled(pool: &SqlitePool, user_id: &str) -> Result<bool, DomainError> {
    Ok(db::totp::is_enabled(pool, user_id).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::test_pool;

    #[tokio::test]
    async fn first_user_becomes_admin() {
        let pool = test_pool().await;
        let user = register(&pool, "admin@example.com", "password123", Some("Admin"))
            .await
            .unwrap();
        assert_eq!(user.role, "admin");
        assert_eq!(user.email, "admin@example.com");
    }

    #[tokio::test]
    async fn second_user_is_not_admin() {
        let pool = test_pool().await;
        register(&pool, "first@example.com", "pass1", None)
            .await
            .unwrap();
        let second = register(&pool, "second@example.com", "pass2", None)
            .await
            .unwrap();
        assert_eq!(second.role, "user");
    }

    #[tokio::test]
    async fn duplicate_email_returns_validation_error() {
        let pool = test_pool().await;
        register(&pool, "alice@example.com", "pass1", None)
            .await
            .unwrap();
        let err = register(&pool, "alice@example.com", "pass2", None)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation("email_taken")));
    }

    #[tokio::test]
    async fn registration_disabled_returns_forbidden() {
        let pool = test_pool().await;
        kartoteka_db::server_config::set(&pool, "registration_enabled", "false")
            .await
            .unwrap();
        let err = register(&pool, "user@example.com", "pass", None)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Forbidden));
    }

    #[tokio::test]
    async fn register_creates_password_auth_method() {
        let pool = test_pool().await;
        let user = register(&pool, "user@example.com", "secret", None)
            .await
            .unwrap();
        let method =
            kartoteka_db::auth_methods::find_by_user_and_provider(&pool, &user.id, "password")
                .await
                .unwrap()
                .unwrap();
        assert_eq!(method.provider, "password");
        assert!(method.credential.as_deref().unwrap().starts_with("$argon2"));
    }

    #[tokio::test]
    async fn hash_and_verify_password_roundtrip() {
        let hash = hash_password("my_password".to_string()).await.unwrap();
        assert!(verify_password("my_password".to_string(), hash.clone())
            .await
            .unwrap());
        assert!(!verify_password("wrong".to_string(), hash).await.unwrap());
    }

    #[tokio::test]
    async fn is_registration_enabled_defaults_true() {
        let pool = test_pool().await;
        assert!(is_registration_enabled(&pool).await.unwrap());
    }

    #[tokio::test]
    async fn is_registration_enabled_false_when_set() {
        let pool = test_pool().await;
        kartoteka_db::server_config::set(&pool, "registration_enabled", "false")
            .await
            .unwrap();
        assert!(!is_registration_enabled(&pool).await.unwrap());
    }

    #[tokio::test]
    async fn setup_totp_returns_secret_and_url() {
        let pool = test_pool().await;
        let user = register(&pool, "user@example.com", "pass", None).await.unwrap();
        let setup = setup_totp(&pool, &user.id, &user.email).await.unwrap();
        assert!(!setup.secret.is_empty());
        assert!(setup.otpauth_url.starts_with("otpauth://totp/"));
        assert!(setup.otpauth_url.contains("Kartoteka"));
    }

    #[tokio::test]
    async fn verify_totp_setup_marks_verified() {
        use totp_rs::{Algorithm, Secret, TOTP};
        let pool = test_pool().await;
        let user = register(&pool, "user@example.com", "pass", None).await.unwrap();
        let setup = setup_totp(&pool, &user.id, &user.email).await.unwrap();
        let bytes = Secret::Encoded(setup.secret.clone()).to_bytes().unwrap();
        let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, bytes, None, String::new()).unwrap();
        let code = totp.generate_current().unwrap();
        verify_totp_setup(&pool, &user.id, &code).await.unwrap();
        assert!(is_totp_enabled(&pool, &user.id).await.unwrap());
    }

    #[tokio::test]
    async fn verify_totp_setup_with_wrong_code_returns_error() {
        let pool = test_pool().await;
        let user = register(&pool, "user@example.com", "pass", None).await.unwrap();
        setup_totp(&pool, &user.id, &user.email).await.unwrap();
        let err = verify_totp_setup(&pool, &user.id, "000000").await.unwrap_err();
        assert!(matches!(err, DomainError::Validation("invalid_totp_code")));
    }

    #[tokio::test]
    async fn disable_totp_removes_secret() {
        use totp_rs::{Algorithm, Secret, TOTP};
        let pool = test_pool().await;
        let user = register(&pool, "user@example.com", "pass", None).await.unwrap();
        let setup = setup_totp(&pool, &user.id, &user.email).await.unwrap();
        let bytes = Secret::Encoded(setup.secret).to_bytes().unwrap();
        let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, bytes, None, String::new()).unwrap();
        let code = totp.generate_current().unwrap();
        verify_totp_setup(&pool, &user.id, &code).await.unwrap();
        disable_totp(&pool, &user.id).await.unwrap();
        assert!(!is_totp_enabled(&pool, &user.id).await.unwrap());
    }

    #[tokio::test]
    async fn check_totp_code_returns_false_when_totp_not_enabled() {
        let pool = test_pool().await;
        let user = register(&pool, "user@example.com", "pass", None).await.unwrap();
        let result = check_totp_code(&pool, &user.id, "123456").await.unwrap();
        assert!(!result);
    }
}
