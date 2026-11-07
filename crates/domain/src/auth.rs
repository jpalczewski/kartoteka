use crate::DomainError;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use kartoteka_db as db;
use serde::Serialize;
use sqlx::SqlitePool;
use std::collections::HashSet;
use totp_rs::{Algorithm as TotpAlgorithm, Builder as TotpBuilder, Secret, Totp};

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

/// Resolved identity from a validated JWT bearer token.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub scope: String,
}

/// Result of token creation — the JWT string is returned once and not stored.
#[derive(Debug, Clone, Serialize)]
pub struct TokenCreated {
    pub id: String,
    pub token: String,
    pub name: String,
    pub scope: String,
}

#[derive(Debug, serde::Deserialize)]
struct JwtClaims {
    sub: String,
    scope: String,
    jti: String,
    #[allow(dead_code)]
    iat: usize,
    #[allow(dead_code)]
    exp: Option<usize>,
}

/// Hash a password using argon2id. Runs in a blocking thread to avoid blocking async executor.
#[tracing::instrument(skip(password))]
pub async fn hash_password(password: String) -> Result<String, DomainError> {
    tokio::task::spawn_blocking(move || {
        let mut salt_bytes = [0u8; 16];
        getrandom::fill(&mut salt_bytes)
            .map_err(|e| DomainError::Internal(format!("getrandom: {e}")))?;
        let salt = SaltString::encode_b64(&salt_bytes)
            .map_err(|e| DomainError::Internal(format!("salt encode: {e}")))?;
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

fn make_totp(secret_b32: &str) -> Result<Totp, DomainError> {
    let secret = Secret::try_from_base32(secret_b32)
        .map_err(|e| DomainError::Internal(format!("totp secret decode: {e}")))?;
    TotpBuilder::new()
        .with_algorithm(TotpAlgorithm::SHA1)
        .with_digits(6)
        .with_skew(1)
        .with_step_duration(30)
        .with_secret(secret)
        .build()
        .map_err(|e| DomainError::Internal(format!("totp init: {e}")))
}

/// Update a server configuration key.
#[tracing::instrument(skip(pool))]
pub async fn set_server_config(
    pool: &SqlitePool,
    key: &str,
    value: &str,
) -> Result<(), DomainError> {
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
    let secret = Secret::generate();
    let totp = TotpBuilder::new()
        .with_algorithm(TotpAlgorithm::SHA1)
        .with_digits(6)
        .with_skew(1)
        .with_step_duration(30)
        .with_secret(secret)
        .with_issuer(Some("Kartoteka"))
        .with_account_name(email)
        .build()
        .map_err(|e| DomainError::Internal(format!("totp init: {e}")))?;
    let secret_b32 = totp.secret().to_base32();
    let url = totp
        .to_url()
        .map_err(|e| DomainError::Internal(format!("totp url: {e}")))?;
    db::totp::upsert(pool, user_id, &secret_b32).await?;
    Ok(TotpSetup {
        secret: secret_b32,
        otpauth_url: url,
    })
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
    if totp.check_current(code).is_none() {
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
    if totp.check_current(code).is_none() {
        return Ok(false);
    }

    // RFC 6238 §5.2: each code must be accepted at most once. With skew=1
    // and step=30s a code stays valid for ~90s; without this guard a leaked
    // code could be replayed throughout that window.
    if !db::totp::try_mark_code_used(pool, user_id, code).await? {
        return Ok(false);
    }
    Ok(true)
}

/// Returns true iff the user has a verified TOTP secret.
#[tracing::instrument(skip(pool))]
pub async fn is_totp_enabled(pool: &SqlitePool, user_id: &str) -> Result<bool, DomainError> {
    Ok(db::totp::is_enabled(pool, user_id).await?)
}

/// Scopes a personal access token may be issued with.
pub const ALLOWED_TOKEN_SCOPES: &[&str] = &["full", "readonly"];
/// Expiry applied when the caller does not request one explicitly.
pub const DEFAULT_TOKEN_TTL_DAYS: i64 = 90;
/// Hard upper bound for a requested token expiry.
pub const MAX_TOKEN_TTL_DAYS: i64 = 365;

/// Input for issuing a personal access token.
///
/// `scope` defaults to `"full"` when omitted. `expires_at` is an RFC3339
/// timestamp; when omitted the token gets [`DEFAULT_TOKEN_TTL_DAYS`] days.
#[derive(Debug, Clone, Default)]
pub struct CreateTokenParams {
    pub name: String,
    pub scope: Option<String>,
    pub expires_at: Option<String>,
}

/// Token parameters after validation, with a concrete expiry.
#[derive(Debug, Clone)]
pub struct ValidatedTokenParams {
    pub name: String,
    pub scope: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Errors from validating or creating a personal access token.
#[derive(Debug, thiserror::Error)]
pub enum CreateTokenError {
    #[error("invalid scope '{0}': must be one of: full, readonly")]
    InvalidScope(String),
    #[error("invalid expires_at: expected RFC3339")]
    InvalidExpiryFormat,
    #[error("expires_at must be in the future")]
    ExpiryInPast,
    #[error("expires_at exceeds maximum of 365 days")]
    ExpiryTooLong,
    #[error(transparent)]
    Domain(#[from] DomainError),
}

/// Validate scope and expiry for a new personal token and normalise the expiry.
///
/// This is the single source of truth for both the REST API and the Leptos
/// server functions, so no caller can mint a token with an off-allowlist scope
/// or an unbounded lifetime. `now` is a parameter to keep the function pure and
/// deterministic in tests.
pub fn validate_create_token(
    params: CreateTokenParams,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<ValidatedTokenParams, CreateTokenError> {
    let scope = params.scope.unwrap_or_else(|| "full".to_string());
    if !ALLOWED_TOKEN_SCOPES.contains(&scope.as_str()) {
        return Err(CreateTokenError::InvalidScope(scope));
    }

    let expires_at = match params.expires_at.as_deref() {
        Some(raw) => {
            let dt = chrono::DateTime::parse_from_rfc3339(raw)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .map_err(|_| CreateTokenError::InvalidExpiryFormat)?;
            if dt <= now {
                return Err(CreateTokenError::ExpiryInPast);
            }
            if dt > now + chrono::Duration::days(MAX_TOKEN_TTL_DAYS) {
                return Err(CreateTokenError::ExpiryTooLong);
            }
            dt
        }
        None => now + chrono::Duration::days(DEFAULT_TOKEN_TTL_DAYS),
    };

    Ok(ValidatedTokenParams {
        name: params.name,
        scope,
        expires_at,
    })
}

/// Create a validated personal access token.
///
/// The scope must be one of [`ALLOWED_TOKEN_SCOPES`] and the expiry defaults to
/// [`DEFAULT_TOKEN_TTL_DAYS`] days, capped at [`MAX_TOKEN_TTL_DAYS`]. Use this
/// (or [`validate_create_token`]) instead of signing a token directly.
#[tracing::instrument(skip(pool, signing_secret, params))]
pub async fn create_personal_token(
    pool: &SqlitePool,
    signing_secret: &str,
    user_id: &str,
    params: CreateTokenParams,
) -> Result<TokenCreated, CreateTokenError> {
    let validated = validate_create_token(params, chrono::Utc::now())?;
    Ok(create_token(
        pool,
        signing_secret,
        user_id,
        &validated.name,
        &validated.scope,
        Some(validated.expires_at),
    )
    .await?)
}

/// Low-level token signing. Does NOT validate scope or expiry — call via
/// [`create_personal_token`]. Generates a UUID jti, signs the claims as JWT
/// HS256, and stores metadata in personal_tokens. The JWT is returned once.
#[tracing::instrument(skip(pool, signing_secret))]
async fn create_token(
    pool: &SqlitePool,
    signing_secret: &str,
    user_id: &str,
    name: &str,
    scope: &str,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<TokenCreated, DomainError> {
    let jti = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp() as usize;

    let mut claims = serde_json::json!({
        "sub": user_id,
        "scope": scope,
        "jti": &jti,
        "iat": now,
    });
    if let Some(exp) = expires_at.map(|dt| dt.timestamp() as usize) {
        claims["exp"] = serde_json::json!(exp);
    }

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(signing_secret.as_bytes()),
    )
    .map_err(|e| DomainError::Internal(format!("jwt encode: {e}")))?;

    let expires_at_str = expires_at.map(|dt| dt.to_rfc3339());
    let row =
        db::personal_tokens::create(pool, &jti, user_id, name, scope, expires_at_str.as_deref())
            .await?;

    Ok(TokenCreated {
        id: row.id,
        token,
        name: row.name,
        scope: row.scope,
    })
}

/// Validate a JWT bearer token and return AuthContext on success.
///
/// For all scopes except "mcp": checks the jti exists in personal_tokens (revocation)
/// and updates last_used_at. For "mcp" scope: skips the db check (short-lived OAuth
/// tokens are not stored in personal_tokens).
#[tracing::instrument(skip(pool, token, signing_secret))]
pub async fn validate_jwt(
    pool: &SqlitePool,
    token: &str,
    signing_secret: &str,
) -> Result<AuthContext, DomainError> {
    let mut validation = Validation::new(Algorithm::HS256);
    // Personal access tokens may legitimately omit exp (user-chosen never-expire).
    // MCP tokens, however, MUST have exp — enforced explicitly below.
    validation.required_spec_claims = HashSet::new();

    let data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(signing_secret.as_bytes()),
        &validation,
    )
    .map_err(|_| DomainError::Validation("invalid_token"))?;

    let claims = data.claims;

    if claims.scope == "mcp" {
        // MCP tokens are not persisted — enforce a hard TTL via exp.
        if claims.exp.is_none() {
            return Err(DomainError::Validation("invalid_token"));
        }
    } else {
        let row = db::personal_tokens::find_by_id(pool, &claims.jti)
            .await?
            .ok_or(DomainError::Validation("token_revoked"))?;

        // Guard against forged tokens with a valid jti but mismatched sub.
        if row.user_id != claims.sub {
            return Err(DomainError::Validation("invalid_token"));
        }

        // Enforce DB-side expiry so revocation via expires_at update works immediately.
        if let Some(exp_str) = &row.expires_at {
            let exp = chrono::DateTime::parse_from_rfc3339(exp_str)
                .map_err(|_| DomainError::Internal("bad expires_at in db".into()))?;
            if exp <= chrono::Utc::now() {
                return Err(DomainError::Validation("token_expired"));
            }
        }

        // Update last_used_at; ignore errors — token is still valid
        let _ = db::personal_tokens::touch_last_used(pool, &claims.jti).await;
    }

    Ok(AuthContext {
        user_id: claims.sub,
        scope: claims.scope,
    })
}

/// List all personal tokens for a user (metadata only, no JWT strings).
#[tracing::instrument(skip(pool))]
pub async fn list_tokens(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<db::personal_tokens::PersonalTokenRow>, DomainError> {
    Ok(db::personal_tokens::list_by_user(pool, user_id).await?)
}

/// Revoke a token by deleting it. Returns NotFound if token doesn't exist or isn't owned.
#[tracing::instrument(skip(pool))]
pub async fn revoke_token(
    pool: &SqlitePool,
    token_id: &str,
    user_id: &str,
) -> Result<(), DomainError> {
    let deleted = db::personal_tokens::delete(pool, token_id, user_id).await?;
    if !deleted {
        return Err(DomainError::NotFound("token"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::test_pool;

    const TEST_PW: &str = "test-placeholder-pw";
    const WRONG_PW: &str = "test-wrong-pw";

    #[tokio::test]
    async fn first_user_becomes_admin() {
        let pool = test_pool().await;
        let user = register(&pool, "admin@example.com", TEST_PW, Some("Admin"))
            .await
            .unwrap();
        assert_eq!(user.role, "admin");
        assert_eq!(user.email, "admin@example.com");
    }

    #[tokio::test]
    async fn second_user_is_not_admin() {
        let pool = test_pool().await;
        register(&pool, "first@example.com", TEST_PW, None)
            .await
            .unwrap();
        let second = register(&pool, "second@example.com", TEST_PW, None)
            .await
            .unwrap();
        assert_eq!(second.role, "user");
    }

    #[tokio::test]
    async fn duplicate_email_returns_validation_error() {
        let pool = test_pool().await;
        register(&pool, "alice@example.com", TEST_PW, None)
            .await
            .unwrap();
        let err = register(&pool, "alice@example.com", TEST_PW, None)
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
        let err = register(&pool, "user@example.com", TEST_PW, None)
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
        let hash = hash_password(TEST_PW.to_string()).await.unwrap();
        assert!(
            verify_password(TEST_PW.to_string(), hash.clone())
                .await
                .unwrap()
        );
        assert!(!verify_password(WRONG_PW.to_string(), hash).await.unwrap());
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
        let user = register(&pool, "user@example.com", TEST_PW, None)
            .await
            .unwrap();
        let setup = setup_totp(&pool, &user.id, &user.email).await.unwrap();
        assert!(!setup.secret.is_empty());
        assert!(setup.otpauth_url.starts_with("otpauth://totp/Kartoteka:"));
        assert!(setup.otpauth_url.contains("issuer=Kartoteka"));
        assert!(
            setup
                .otpauth_url
                .contains(&format!("secret={}", setup.secret))
        );
    }

    #[test]
    fn make_totp_round_trips_stored_base32_secret() {
        // Secrets written by earlier totp-rs 5.x releases are uppercase, unpadded
        // RFC 4648 base32; they must keep decoding to the same bytes and
        // re-encode to the identical string, otherwise existing 2FA enrolments
        // would stop verifying.
        let stored = "OBWGC2LOFVZXI4TJNZTS243FMNZGK5BNGEZDG";
        let totp = make_totp(stored).unwrap();
        assert_eq!(totp.secret().to_base32(), stored);
        assert_eq!(totp.secret().as_bytes().len(), 23);
    }

    #[tokio::test]
    async fn verify_totp_setup_marks_verified() {
        use totp_rs::{Algorithm, Builder, Secret};
        let pool = test_pool().await;
        let user = register(&pool, "user@example.com", TEST_PW, None)
            .await
            .unwrap();
        let setup = setup_totp(&pool, &user.id, &user.email).await.unwrap();
        let secret = Secret::try_from_base32(setup.secret.clone()).unwrap();
        let totp = Builder::new()
            .with_algorithm(Algorithm::SHA1)
            .with_digits(6)
            .with_skew(1)
            .with_step_duration(30)
            .with_secret(secret)
            .build()
            .unwrap();
        let code = totp.generate_current().to_string();
        verify_totp_setup(&pool, &user.id, &code).await.unwrap();
        assert!(is_totp_enabled(&pool, &user.id).await.unwrap());
    }

    #[tokio::test]
    async fn verify_totp_setup_with_wrong_code_returns_error() {
        let pool = test_pool().await;
        let user = register(&pool, "user@example.com", TEST_PW, None)
            .await
            .unwrap();
        setup_totp(&pool, &user.id, &user.email).await.unwrap();
        let err = verify_totp_setup(&pool, &user.id, "000000")
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Validation("invalid_totp_code")));
    }

    #[tokio::test]
    async fn disable_totp_removes_secret() {
        use totp_rs::{Algorithm, Builder, Secret};
        let pool = test_pool().await;
        let user = register(&pool, "user@example.com", TEST_PW, None)
            .await
            .unwrap();
        let setup = setup_totp(&pool, &user.id, &user.email).await.unwrap();
        let secret = Secret::try_from_base32(setup.secret).unwrap();
        let totp = Builder::new()
            .with_algorithm(Algorithm::SHA1)
            .with_digits(6)
            .with_skew(1)
            .with_step_duration(30)
            .with_secret(secret)
            .build()
            .unwrap();
        let code = totp.generate_current().to_string();
        verify_totp_setup(&pool, &user.id, &code).await.unwrap();
        disable_totp(&pool, &user.id).await.unwrap();
        assert!(!is_totp_enabled(&pool, &user.id).await.unwrap());
    }

    #[tokio::test]
    async fn check_totp_code_returns_false_when_totp_not_enabled() {
        let pool = test_pool().await;
        let user = register(&pool, "user@example.com", TEST_PW, None)
            .await
            .unwrap();
        let result = check_totp_code(&pool, &user.id, "123456").await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn check_totp_code_rejects_replay() {
        // A valid code that is accepted once must be rejected on the second use,
        // even while still inside the TOTP validity window.
        let pool = test_pool().await;
        let user = register(&pool, "user@example.com", TEST_PW, None)
            .await
            .unwrap();
        let setup = setup_totp(&pool, &user.id, "user@example.com")
            .await
            .unwrap();
        kartoteka_db::totp::mark_verified(&pool, &user.id)
            .await
            .unwrap();

        let code = make_totp(&setup.secret)
            .unwrap()
            .generate_current()
            .to_string();
        assert!(check_totp_code(&pool, &user.id, &code).await.unwrap());
        assert!(
            !check_totp_code(&pool, &user.id, &code).await.unwrap(),
            "second use of the same TOTP code must be rejected"
        );
    }

    // ── C3: JWT token tests ──────────────────────────────────────────────

    const TEST_SECRET: &str = "test-signing-secret-32-bytes-min!!";

    fn params(name: &str, scope: &str) -> CreateTokenParams {
        CreateTokenParams {
            name: name.to_string(),
            scope: Some(scope.to_string()),
            expires_at: None,
        }
    }

    #[test]
    fn validate_create_token_rejects_unknown_scope() {
        let now = chrono::Utc::now();
        let err = validate_create_token(params("T", "api"), now).unwrap_err();
        assert!(matches!(err, CreateTokenError::InvalidScope(ref s) if s == "api"));
    }

    #[test]
    fn validate_create_token_accepts_full_and_readonly() {
        let now = chrono::Utc::now();
        for scope in ["full", "readonly"] {
            let validated = validate_create_token(params("T", scope), now).unwrap();
            assert_eq!(validated.scope, scope);
        }
    }

    #[test]
    fn validate_create_token_defaults_to_full_scope() {
        let now = chrono::Utc::now();
        let validated = validate_create_token(
            CreateTokenParams {
                name: "T".to_string(),
                scope: None,
                expires_at: None,
            },
            now,
        )
        .unwrap();
        assert_eq!(validated.scope, "full");
    }

    #[test]
    fn validate_create_token_defaults_expiry_to_90_days() {
        let now = chrono::Utc::now();
        let validated = validate_create_token(params("T", "full"), now).unwrap();
        assert_eq!(validated.expires_at, now + chrono::Duration::days(90));
    }

    #[test]
    fn validate_create_token_rejects_expiry_over_max() {
        let now = chrono::Utc::now();
        let too_far = (now + chrono::Duration::days(366)).to_rfc3339();
        let err = validate_create_token(
            CreateTokenParams {
                name: "T".to_string(),
                scope: Some("full".to_string()),
                expires_at: Some(too_far),
            },
            now,
        )
        .unwrap_err();
        assert!(matches!(err, CreateTokenError::ExpiryTooLong));
    }

    #[test]
    fn validate_create_token_rejects_past_expiry() {
        let now = chrono::Utc::now();
        let past = (now - chrono::Duration::days(1)).to_rfc3339();
        let err = validate_create_token(
            CreateTokenParams {
                name: "T".to_string(),
                scope: Some("readonly".to_string()),
                expires_at: Some(past),
            },
            now,
        )
        .unwrap_err();
        assert!(matches!(err, CreateTokenError::ExpiryInPast));
    }

    #[test]
    fn validate_create_token_rejects_malformed_expiry() {
        let now = chrono::Utc::now();
        let err = validate_create_token(
            CreateTokenParams {
                name: "T".to_string(),
                scope: Some("full".to_string()),
                expires_at: Some("tomorrow".to_string()),
            },
            now,
        )
        .unwrap_err();
        assert!(matches!(err, CreateTokenError::InvalidExpiryFormat));
    }

    #[tokio::test]
    async fn create_personal_token_rejects_bad_scope() {
        let pool = test_pool().await;
        let user = register(&pool, "user@example.com", TEST_PW, None)
            .await
            .unwrap();
        let err = create_personal_token(&pool, TEST_SECRET, &user.id, params("T", "api"))
            .await
            .unwrap_err();
        assert!(matches!(err, CreateTokenError::InvalidScope(_)));
        // Nothing must have been persisted.
        assert!(list_tokens(&pool, &user.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn create_personal_token_stores_default_expiry() {
        let pool = test_pool().await;
        let user = register(&pool, "user@example.com", TEST_PW, None)
            .await
            .unwrap();
        let before = chrono::Utc::now();
        create_personal_token(&pool, TEST_SECRET, &user.id, params("T", "full"))
            .await
            .unwrap();
        let tokens = list_tokens(&pool, &user.id).await.unwrap();
        assert_eq!(tokens.len(), 1);
        let expires_at = tokens[0]
            .expires_at
            .as_deref()
            .expect("token must have an expiry");
        let expires_at = chrono::DateTime::parse_from_rfc3339(expires_at).unwrap();
        let lower =
            before + chrono::Duration::days(DEFAULT_TOKEN_TTL_DAYS) - chrono::Duration::seconds(5);
        let upper = chrono::Utc::now()
            + chrono::Duration::days(DEFAULT_TOKEN_TTL_DAYS)
            + chrono::Duration::seconds(5);
        assert!(expires_at >= lower && expires_at <= upper);
    }

    #[tokio::test]
    async fn create_token_returns_jwt_and_row() {
        let pool = test_pool().await;
        let user = register(&pool, "user@example.com", TEST_PW, None)
            .await
            .unwrap();
        let created =
            create_personal_token(&pool, TEST_SECRET, &user.id, params("My Token", "full"))
                .await
                .unwrap();
        assert!(!created.token.is_empty());
        assert_eq!(created.name, "My Token");
        assert_eq!(created.scope, "full");
        assert!(!created.id.is_empty());
        // JWT must decode correctly
        let ctx = validate_jwt(&pool, &created.token, TEST_SECRET)
            .await
            .unwrap();
        assert_eq!(ctx.user_id, user.id);
        assert_eq!(ctx.scope, "full");
    }

    #[tokio::test]
    async fn validate_jwt_with_wrong_secret_fails() {
        let pool = test_pool().await;
        let user = register(&pool, "user@example.com", TEST_PW, None)
            .await
            .unwrap();
        let created = create_personal_token(&pool, TEST_SECRET, &user.id, params("Token", "full"))
            .await
            .unwrap();
        let result = validate_jwt(&pool, &created.token, "wrong-secret-32-bytes-min!!!!!!").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn validate_jwt_revoked_token_fails() {
        let pool = test_pool().await;
        let user = register(&pool, "user@example.com", TEST_PW, None)
            .await
            .unwrap();
        let created = create_personal_token(&pool, TEST_SECRET, &user.id, params("Token", "full"))
            .await
            .unwrap();
        revoke_token(&pool, &created.id, &user.id).await.unwrap();
        let result = validate_jwt(&pool, &created.token, TEST_SECRET).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_tokens_returns_user_tokens() {
        let pool = test_pool().await;
        let user = register(&pool, "user@example.com", TEST_PW, None)
            .await
            .unwrap();
        create_personal_token(&pool, TEST_SECRET, &user.id, params("Token A", "full"))
            .await
            .unwrap();
        create_personal_token(&pool, TEST_SECRET, &user.id, params("Token B", "readonly"))
            .await
            .unwrap();
        let tokens = list_tokens(&pool, &user.id).await.unwrap();
        assert_eq!(tokens.len(), 2);
    }

    #[tokio::test]
    async fn revoke_token_not_owned_returns_not_found() {
        let pool = test_pool().await;
        let user1 = register(&pool, "u1@example.com", TEST_PW, None)
            .await
            .unwrap();
        let user2 = register(&pool, "u2@example.com", TEST_PW, None)
            .await
            .unwrap();
        let created = create_personal_token(&pool, TEST_SECRET, &user1.id, params("Token", "full"))
            .await
            .unwrap();
        let err = revoke_token(&pool, &created.id, &user2.id)
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound("token")));
    }

    #[tokio::test]
    async fn validate_jwt_mcp_scope_skips_revocation_check_but_requires_exp() {
        // MCP tokens (short-lived, issued by OAuth) are not stored in personal_tokens,
        // so they skip the DB revocation check — but they MUST carry an exp claim to
        // avoid becoming permanently valid.
        let pool = test_pool().await;
        use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

        // Without exp: must be rejected.
        let claims_no_exp = serde_json::json!({
            "sub": "some-user-id",
            "scope": "mcp",
            "jti": "phantom-jti-not-in-db",
            "iat": 0usize,
        });
        let token_no_exp = encode(
            &Header::new(Algorithm::HS256),
            &claims_no_exp,
            &EncodingKey::from_secret(TEST_SECRET.as_bytes()),
        )
        .unwrap();
        assert!(
            validate_jwt(&pool, &token_no_exp, TEST_SECRET)
                .await
                .is_err()
        );

        // With future exp: passes without DB lookup.
        let future_exp = (chrono::Utc::now().timestamp() + 3600) as usize;
        let claims_ok = serde_json::json!({
            "sub": "some-user-id",
            "scope": "mcp",
            "jti": "phantom-jti-not-in-db",
            "iat": 0usize,
            "exp": future_exp,
        });
        let token_ok = encode(
            &Header::new(Algorithm::HS256),
            &claims_ok,
            &EncodingKey::from_secret(TEST_SECRET.as_bytes()),
        )
        .unwrap();
        let ctx = validate_jwt(&pool, &token_ok, TEST_SECRET).await.unwrap();
        assert_eq!(ctx.scope, "mcp");
    }

    #[tokio::test]
    async fn validate_jwt_rejects_sub_mismatch() {
        // Cross-check: a forged token whose jti points to user A but sub claims user B
        // must be rejected even though the jti is present in personal_tokens.
        let pool = test_pool().await;
        let user_a = register(&pool, "a@example.com", TEST_PW, None)
            .await
            .unwrap();
        let created =
            create_personal_token(&pool, TEST_SECRET, &user_a.id, params("Token", "full"))
                .await
                .unwrap();

        // Forge a JWT with user_a's jti but a different sub.
        use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
        let claims = serde_json::json!({
            "sub": "attacker-id",
            "scope": "full",
            "jti": &created.id,
            "iat": 0usize,
        });
        let forged = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(TEST_SECRET.as_bytes()),
        )
        .unwrap();
        assert!(validate_jwt(&pool, &forged, TEST_SECRET).await.is_err());
    }
}
