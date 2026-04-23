#![allow(clippy::too_many_arguments)]

use axum::{
    Json,
    extract::{Form, Query, State},
    response::{IntoResponse, Redirect, Response},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration, Utc};
use kartoteka_db::oauth::{
    clients as oauth_clients, codes as oauth_codes, refresh as oauth_refresh,
};
use rand::RngCore;
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use url::Url;
use uuid::Uuid;

use crate::{OAuthState, errors::OAuthError, pkce, storage, types::*};

// ── Well-known metadata ──────────────────────────────────────────────────────

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

pub async fn metadata_pr(State(s): State<OAuthState>) -> Json<ProtectedResourceMetadata> {
    let base = &s.public_base_url;
    Json(ProtectedResourceMetadata {
        resource: format!("{base}/mcp"),
        authorization_servers: vec![base.clone()],
        bearer_methods_supported: vec!["header"],
        scopes_supported: vec!["mcp"],
    })
}

// ── DCR ──────────────────────────────────────────────────────────────────────

pub async fn register(
    State(s): State<OAuthState>,
    Json(req): Json<DcrRequest>,
) -> Result<Json<DcrResponse>, OAuthError> {
    if req.redirect_uris.is_empty() {
        return Err(OAuthError::InvalidRequest(
            "redirect_uris must not be empty",
        ));
    }
    for uri in &req.redirect_uris {
        Url::parse(uri)
            .map_err(|_| OAuthError::InvalidRequest("redirect_uri is not a valid absolute URL"))?;
    }
    if let Some(m) = &req.token_endpoint_auth_method {
        if m != "none" {
            return Err(OAuthError::InvalidRequest(
                "token_endpoint_auth_method must be \"none\"",
            ));
        }
    }

    let client_id = Uuid::new_v4().to_string();
    let redirect_uris_json = serde_json::to_string(&req.redirect_uris)
        .map_err(|e| OAuthError::Internal(e.to_string()))?;
    oauth_clients::create(&s.pool, &client_id, &req.client_name, &redirect_uris_json).await?;

    Ok(Json(DcrResponse {
        client_id,
        client_name: req.client_name,
        redirect_uris: req.redirect_uris,
        token_endpoint_auth_method: "none",
        grant_types: vec!["authorization_code", "refresh_token"],
        response_types: vec!["code"],
    }))
}

// ── Authorization ─────────────────────────────────────────────────────────────

pub async fn authorize_get(
    State(s): State<OAuthState>,
    session: tower_sessions::Session,
    auth: axum_login::AuthSession<kartoteka_auth::KartotekaBackend>,
    Query(params): Query<AuthorizeParams>,
) -> Result<Response, OAuthError> {
    if params.response_type != "code" {
        return Err(OAuthError::InvalidRequest("response_type must be \"code\""));
    }
    if params.code_challenge_method != "S256" {
        return Err(OAuthError::InvalidRequest(
            "code_challenge_method must be \"S256\"",
        ));
    }
    if params.code_challenge.len() < 43 || params.code_challenge.len() > 128 {
        return Err(OAuthError::InvalidRequest(
            "code_challenge length must be 43-128",
        ));
    }
    if params.scope != "mcp" {
        return Err(OAuthError::InvalidRequest("unsupported scope"));
    }
    if params.state.is_empty() {
        return Err(OAuthError::InvalidRequest("state is required"));
    }

    let client = oauth_clients::find(&s.pool, &params.client_id)
        .await?
        .ok_or(OAuthError::InvalidClient)?;
    let registered: Vec<String> = serde_json::from_str(&client.redirect_uris)
        .map_err(|e| OAuthError::Internal(e.to_string()))?;
    if !registered.contains(&params.redirect_uri) {
        return Err(OAuthError::InvalidRequest(
            "redirect_uri does not match registered",
        ));
    }

    if auth.user.is_none() {
        let qs = serde_urlencoded::to_string(&params).unwrap_or_default();
        let return_to = format!("/oauth/authorize?{qs}");
        let target = format!("/login?return_to={}", urlencoding::encode(&return_to));
        return Ok(Redirect::to(&target).into_response());
    }

    let mut csrf_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut csrf_bytes);
    let csrf_token = URL_SAFE_NO_PAD.encode(csrf_bytes);

    let pending = PendingOAuthRequest {
        client_id: params.client_id,
        redirect_uri: params.redirect_uri,
        scope: params.scope,
        state: params.state,
        code_challenge: params.code_challenge,
        csrf_token,
        created_at: Utc::now(),
    };
    session
        .insert("pending_oauth_request", &pending)
        .await
        .map_err(|e| OAuthError::Internal(e.to_string()))?;

    Ok(Redirect::to("/consent").into_response())
}

const CONSENT_TTL_MIN: i64 = 10;
const AUTH_CODE_TTL_MIN: i64 = 5;

pub async fn authorize_post(
    State(s): State<OAuthState>,
    session: tower_sessions::Session,
    auth: axum_login::AuthSession<kartoteka_auth::KartotekaBackend>,
    Form(form): Form<ConsentForm>,
) -> Result<Redirect, OAuthError> {
    let user = auth.user.ok_or(OAuthError::AccessDenied)?;

    let pending: PendingOAuthRequest = session
        .get("pending_oauth_request")
        .await
        .map_err(|e| OAuthError::Internal(e.to_string()))?
        .ok_or(OAuthError::InvalidRequest(
            "no pending authorization request",
        ))?;

    if Utc::now() - pending.created_at > Duration::minutes(CONSENT_TTL_MIN) {
        session
            .remove::<PendingOAuthRequest>("pending_oauth_request")
            .await
            .ok();
        return Err(OAuthError::InvalidRequest("authorization request expired"));
    }

    let a = form.csrf_token.as_bytes();
    let b = pending.csrf_token.as_bytes();
    if !bool::from(a.ct_eq(b)) {
        return Err(OAuthError::InvalidRequest("csrf_token mismatch"));
    }

    session
        .remove::<PendingOAuthRequest>("pending_oauth_request")
        .await
        .ok();

    let base_redirect = pending.redirect_uri;
    let state = pending.state;

    if form.decision == "deny" {
        return Ok(Redirect::to(&format!(
            "{base_redirect}?error=access_denied&state={}",
            urlencoding::encode(&state)
        )));
    }
    if form.decision != "approve" {
        return Err(OAuthError::InvalidRequest(
            "decision must be approve or deny",
        ));
    }

    let mut code_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut code_bytes);
    let code = URL_SAFE_NO_PAD.encode(code_bytes);
    let expires_at = Utc::now() + Duration::minutes(AUTH_CODE_TTL_MIN);

    oauth_codes::insert(
        &s.pool,
        &code,
        &pending.client_id,
        &user.id,
        &pending.code_challenge,
        &pending.scope,
        &base_redirect,
        expires_at,
    )
    .await?;

    Ok(Redirect::to(&format!(
        "{base_redirect}?code={}&state={}",
        urlencoding::encode(&code),
        urlencoding::encode(&state),
    )))
}

// ── Token ─────────────────────────────────────────────────────────────────────

const REFRESH_TTL_DAYS: i64 = 30;

pub async fn token(
    State(s): State<OAuthState>,
    Form(form): Form<JsonValue>,
) -> Result<Json<TokenResponse>, OAuthError> {
    let grant_type = form
        .get("grant_type")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    match grant_type {
        "authorization_code" => token_authorization_code(s, form).await,
        "refresh_token" => token_refresh(s, form).await,
        _ => Err(OAuthError::UnsupportedGrantType),
    }
}

async fn token_authorization_code(
    s: OAuthState,
    form: JsonValue,
) -> Result<Json<TokenResponse>, OAuthError> {
    let code = form
        .get("code")
        .and_then(|v| v.as_str())
        .ok_or(OAuthError::InvalidRequest("missing code"))?;
    let redirect_uri = form
        .get("redirect_uri")
        .and_then(|v| v.as_str())
        .ok_or(OAuthError::InvalidRequest("missing redirect_uri"))?;
    let client_id = form
        .get("client_id")
        .and_then(|v| v.as_str())
        .ok_or(OAuthError::InvalidRequest("missing client_id"))?;
    let code_verifier = form
        .get("code_verifier")
        .and_then(|v| v.as_str())
        .ok_or(OAuthError::InvalidRequest("missing code_verifier"))?;

    let row = oauth_codes::consume(&s.pool, code, client_id)
        .await?
        .ok_or(OAuthError::InvalidGrant(
            "unknown, replayed, or expired code",
        ))?;

    if row.redirect_uri != redirect_uri {
        return Err(OAuthError::InvalidGrant(
            "redirect_uri does not match authorization",
        ));
    }
    if !pkce::verify_s256(code_verifier, &row.code_challenge) {
        return Err(OAuthError::InvalidGrant("PKCE verification failed"));
    }

    let access_token = storage::sign_access_token(&row.user_id, &row.scope, &s.signing_secret)?;
    let (refresh_token, refresh_hash) = mint_refresh_token();
    let expires_at = Utc::now() + Duration::days(REFRESH_TTL_DAYS);
    oauth_refresh::insert(
        &s.pool,
        &refresh_hash,
        client_id,
        &row.user_id,
        &row.scope,
        expires_at,
    )
    .await?;

    Ok(Json(TokenResponse {
        access_token,
        token_type: "Bearer",
        expires_in: 3600,
        refresh_token,
        scope: row.scope,
    }))
}

async fn token_refresh(s: OAuthState, form: JsonValue) -> Result<Json<TokenResponse>, OAuthError> {
    let refresh_token = form
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .ok_or(OAuthError::InvalidRequest("missing refresh_token"))?;
    let client_id = form
        .get("client_id")
        .and_then(|v| v.as_str())
        .ok_or(OAuthError::InvalidRequest("missing client_id"))?;

    let hash = URL_SAFE_NO_PAD.encode(Sha256::digest(refresh_token.as_bytes()));
    let row = oauth_refresh::find_and_delete(&s.pool, &hash)
        .await?
        .ok_or(OAuthError::InvalidGrant(
            "unknown, reused, or expired refresh_token",
        ))?;

    if row.client_id != client_id {
        return Err(OAuthError::InvalidGrant("client_id mismatch"));
    }

    let access_token = storage::sign_access_token(&row.user_id, &row.scope, &s.signing_secret)?;
    let (new_refresh, new_hash) = mint_refresh_token();

    oauth_refresh::insert(
        &s.pool,
        &new_hash,
        client_id,
        &row.user_id,
        &row.scope,
        row.expires_at,
    )
    .await?;

    Ok(Json(TokenResponse {
        access_token,
        token_type: "Bearer",
        expires_in: 3600,
        refresh_token: new_refresh,
        scope: row.scope,
    }))
}

fn mint_refresh_token() -> (String, String) {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let token = URL_SAFE_NO_PAD.encode(bytes);
    let hash = URL_SAFE_NO_PAD.encode(Sha256::digest(token.as_bytes()));
    (token, hash)
}
