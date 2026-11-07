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

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
