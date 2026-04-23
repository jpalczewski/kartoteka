use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorizeParams {
    pub client_id: String,
    pub redirect_uri: String,
    pub response_type: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub scope: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingOAuthRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub state: String,
    pub code_challenge: String,
    pub csrf_token: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ConsentForm {
    pub decision: String,
    pub csrf_token: String,
}

#[derive(Debug, Deserialize)]
pub struct DcrRequest {
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    #[serde(default)]
    pub token_endpoint_auth_method: Option<String>,
    #[serde(default)]
    pub grant_types: Option<Vec<String>>,
    #[serde(default)]
    pub response_types: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct DcrResponse {
    pub client_id: String,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub token_endpoint_auth_method: &'static str,
    pub grant_types: Vec<&'static str>,
    pub response_types: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
    pub refresh_token: String,
    pub scope: String,
}

#[derive(Debug, Serialize)]
pub struct AuthServerMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub registration_endpoint: String,
    pub response_types_supported: Vec<&'static str>,
    pub grant_types_supported: Vec<&'static str>,
    pub code_challenge_methods_supported: Vec<&'static str>,
    pub token_endpoint_auth_methods_supported: Vec<&'static str>,
    pub scopes_supported: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct ProtectedResourceMetadata {
    pub resource: String,
    pub authorization_servers: Vec<String>,
    pub bearer_methods_supported: Vec<&'static str>,
    pub scopes_supported: Vec<&'static str>,
}
