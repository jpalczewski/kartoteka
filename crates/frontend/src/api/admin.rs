// All functions in this module are called from #[cfg(target_arch = "wasm32")] blocks.
// Suppress dead_code warnings on non-wasm targets (e.g. CI native clippy).
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use kartoteka_shared::{
    CreateInvitationCodeRequest, InvitationCode, MeResponse, RegistrationModeResponse,
    UpsertSettingRequest, UserSetting, ValidateInviteResponse,
};

use super::{API_BASE, ApiError, HttpClient, api_delete, api_get, api_post, api_put};

/// GET /api/me — creates user row if needed, returns admin flag.
/// Pass `invite_code` to finalize invite code after signup.
pub async fn get_me(
    client: &impl HttpClient,
    invite_code: Option<&str>,
) -> Result<MeResponse, ApiError> {
    let url = match invite_code {
        Some(code) => format!("{API_BASE}/me?invite_code={code}"),
        None => format!("{API_BASE}/me"),
    };
    api_get(client, &url).await
}

/// GET /api/public/registration-mode — no auth, usable before login.
pub async fn get_registration_mode(
    client: &impl HttpClient,
) -> Result<RegistrationModeResponse, ApiError> {
    api_get(client, &format!("{API_BASE}/public/registration-mode")).await
}

/// POST /api/public/validate-invite — validates an invite code (and reserves it).
pub async fn validate_invite(
    client: &impl HttpClient,
    code: &str,
    email: &str,
) -> Result<ValidateInviteResponse, ApiError> {
    api_post(
        client,
        &format!("{API_BASE}/public/validate-invite"),
        &serde_json::json!({ "code": code, "email": email }),
    )
    .await
}

// ── Admin: Instance Settings ───────────────────────────────────────────────

pub async fn list_instance_settings(
    client: &impl HttpClient,
) -> Result<Vec<UserSetting>, ApiError> {
    api_get(client, &format!("{API_BASE}/admin/instance-settings")).await
}

pub async fn update_instance_setting(
    client: &impl HttpClient,
    key: &str,
    value: serde_json::Value,
) -> Result<UserSetting, ApiError> {
    api_put(
        client,
        &format!("{API_BASE}/admin/instance-settings/{key}"),
        &UpsertSettingRequest { value },
    )
    .await
}

// ── Admin: Invitation Codes ────────────────────────────────────────────────

pub async fn list_invitation_codes(
    client: &impl HttpClient,
) -> Result<Vec<InvitationCode>, ApiError> {
    api_get(client, &format!("{API_BASE}/admin/invitation-codes")).await
}

pub async fn create_invitation_code(
    client: &impl HttpClient,
    expires_at: Option<String>,
) -> Result<InvitationCode, ApiError> {
    api_post(
        client,
        &format!("{API_BASE}/admin/invitation-codes"),
        &CreateInvitationCodeRequest { expires_at },
    )
    .await
}

pub async fn delete_invitation_code(client: &impl HttpClient, id: &str) -> Result<(), ApiError> {
    api_delete(client, &format!("{API_BASE}/admin/invitation-codes/{id}")).await
}
