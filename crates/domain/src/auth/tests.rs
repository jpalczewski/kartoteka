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
    let method = kartoteka_db::auth_methods::find_by_user_and_provider(&pool, &user.id, "password")
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
    let created = create_personal_token(&pool, TEST_SECRET, &user.id, params("My Token", "full"))
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
    let created = create_personal_token(&pool, TEST_SECRET, &user_a.id, params("Token", "full"))
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
