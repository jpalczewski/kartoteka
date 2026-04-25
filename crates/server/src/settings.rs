use crate::{AppError, AppState, UserId};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use kartoteka_domain::preferences::UpdatePreferencesRequest;
use serde::Deserialize;

#[derive(Deserialize)]
struct SetValueRequest {
    value: String,
}

/// Raw key-value settings. Mounted under `/settings`.
pub fn settings_router() -> Router<AppState> {
    Router::new().route("/", get(list_settings)).route(
        "/{key}",
        get(get_setting).put(set_setting).delete(delete_setting),
    )
}

/// Typed preferences (timezone + locale). Mounted under `/preferences`.
pub fn preferences_router() -> Router<AppState> {
    Router::new().route("/", get(get_preferences).put(update_preferences))
}

#[tracing::instrument(skip_all, fields(action = "list_settings"))]
async fn list_settings(
    State(state): State<AppState>,
    UserId(uid): UserId,
) -> Result<impl IntoResponse, AppError> {
    let settings = kartoteka_domain::settings::list_all(&state.pool, &uid).await?;
    Ok(Json(settings))
}

#[tracing::instrument(skip_all, fields(action = "get_setting", setting_key = %key))]
async fn get_setting(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(key): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::settings::get(&state.pool, &uid, &key)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound("setting"))
}

#[tracing::instrument(skip_all, fields(action = "set_setting", setting_key = %key))]
async fn set_setting(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(key): Path<String>,
    Json(req): Json<SetValueRequest>,
) -> Result<impl IntoResponse, AppError> {
    let setting = kartoteka_domain::settings::set(&state.pool, &uid, &key, &req.value).await?;
    Ok(Json(setting))
}

#[tracing::instrument(skip_all, fields(action = "delete_setting", setting_key = %key))]
async fn delete_setting(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(key): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    if kartoteka_domain::settings::delete(&state.pool, &uid, &key).await? {
        Ok(StatusCode::NO_CONTENT.into_response())
    } else {
        Err(AppError::NotFound("setting"))
    }
}

#[tracing::instrument(skip_all, fields(action = "get_preferences"))]
async fn get_preferences(
    State(state): State<AppState>,
    UserId(uid): UserId,
) -> Result<impl IntoResponse, AppError> {
    let prefs = kartoteka_domain::preferences::get(&state.pool, &uid).await?;
    Ok(Json(prefs))
}

#[tracing::instrument(skip_all, fields(action = "update_preferences"))]
async fn update_preferences(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Json(req): Json<UpdatePreferencesRequest>,
) -> Result<impl IntoResponse, AppError> {
    let prefs = kartoteka_domain::preferences::update(&state.pool, &uid, &req).await?;
    Ok(Json(prefs))
}
