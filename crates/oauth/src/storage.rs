use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const ACCESS_TOKEN_TTL_SECS: i64 = 3600;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub scope: String,
    pub jti: String,
    pub iat: usize,
    pub exp: usize,
}

pub fn sign_access_token(
    user_id: &str,
    scope: &str,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.into(),
        scope: scope.into(),
        jti: Uuid::new_v4().to_string(),
        iat: now.timestamp() as usize,
        exp: (now + Duration::seconds(ACCESS_TOKEN_TTL_SECS)).timestamp() as usize,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn verify_access_token(
    token: &str,
    secret: &str,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut v = Validation::new(Algorithm::HS256);
    v.set_required_spec_claims(&["sub", "exp", "jti"]);
    decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &v).map(|d| d.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_then_verify_round_trip() {
        let t =
            sign_access_token("user-123", "mcp", "secret-at-least-32-chars-long-padding").unwrap();
        let c = verify_access_token(&t, "secret-at-least-32-chars-long-padding").unwrap();
        assert_eq!(c.sub, "user-123");
        assert_eq!(c.scope, "mcp");
        assert!(!c.jti.is_empty());
        assert!(c.exp > c.iat);
    }

    #[test]
    fn verify_rejects_wrong_secret() {
        let t = sign_access_token("u", "mcp", "secret-at-least-32-chars-long-padding").unwrap();
        assert!(verify_access_token(&t, "different-secret-also-32-chars-lngpad").is_err());
    }
}
