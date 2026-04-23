//! OAuth 2.1 handlers: metadata endpoints (RFC 8414, RFC 9728) + stubs for authorization/token/registration.

use crate::{OAuthState, types::*};
use axum::{Json, extract::State};

/// RFC 8414: Authorization Server Metadata endpoint (/.well-known/oauth-authorization-server)
pub async fn metadata_as(State(s): State<OAuthState>) -> Json<AuthServerMetadata> {
    let base = &s.public_base_url;
    Json(AuthServerMetadata {
        issuer: base.clone(),
        authorization_endpoint: format!("{base}/oauth/authorize"),
        token_endpoint: format!("{base}/oauth/token"),
        registration_endpoint: format!("{base}/oauth/register"),
        response_types_supported: vec!["code"],
        grant_types_supported: vec!["authorization_code", "refresh_token"],
        code_challenge_methods_supported: vec!["S256"],
        token_endpoint_auth_methods_supported: vec!["none"],
        scopes_supported: vec!["mcp"],
    })
}

/// RFC 9728: Protected Resource Metadata endpoint (/.well-known/oauth-protected-resource)
pub async fn metadata_pr(State(s): State<OAuthState>) -> Json<ProtectedResourceMetadata> {
    let base = &s.public_base_url;
    Json(ProtectedResourceMetadata {
        resource: format!("{base}/mcp"),
        authorization_servers: vec![base.clone()],
        bearer_methods_supported: vec!["header"],
        scopes_supported: vec!["mcp"],
    })
}

// Stubs — implemented in later tasks

/// Dynamic Client Registration (DCR) — RFC 7591
pub async fn register() -> &'static str {
    "TODO"
}

/// Authorization endpoint GET — RFC 6749 + PKCE S256
pub async fn authorize_get() -> &'static str {
    "TODO"
}

/// Authorization endpoint POST — RFC 6749
pub async fn authorize_post() -> &'static str {
    "TODO"
}

/// Token endpoint — RFC 6749 authorization_code + refresh_token
pub async fn token() -> &'static str {
    "TODO"
}
