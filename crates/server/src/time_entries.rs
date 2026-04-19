use crate::{AppError, AppState, UserId};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
};

#[derive(serde::Deserialize)]
struct ItemQuery {
    item_id: String,
}

#[derive(serde::Deserialize)]
struct StartRequest {
    item_id: Option<String>,
}

#[derive(serde::Deserialize)]
struct LogRequest {
    item_id: Option<String>,
    started_at: String,
    ended_at: String,
    description: Option<String>,
}

#[derive(serde::Deserialize)]
struct AssignRequest {
    item_id: String,
}

pub fn time_entries_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_for_item))
        .route("/inbox", get(list_inbox))
        .route("/start", post(start_timer))
        .route("/stop", post(stop_timer))
        .route("/log", post(log_time))
        .route("/{id}/assign", patch(assign_entry))
        .route("/{id}", delete(delete_entry))
}

#[tracing::instrument(skip_all, fields(action = "list_time_entries", item_id = %q.item_id))]
async fn list_for_item(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Query(q): Query<ItemQuery>,
) -> Result<impl IntoResponse, AppError> {
    let entries =
        kartoteka_domain::time_entries::list_for_item(&state.pool, &uid, &q.item_id).await?;
    Ok(Json(entries))
}

#[tracing::instrument(skip_all, fields(action = "list_inbox"))]
async fn list_inbox(
    State(state): State<AppState>,
    UserId(uid): UserId,
) -> Result<impl IntoResponse, AppError> {
    let entries = kartoteka_domain::time_entries::list_inbox(&state.pool, &uid).await?;
    Ok(Json(entries))
}

#[tracing::instrument(skip_all, fields(action = "start_timer"))]
async fn start_timer(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Json(req): Json<StartRequest>,
) -> Result<impl IntoResponse, AppError> {
    let entry =
        kartoteka_domain::time_entries::start(&state.pool, &uid, req.item_id.as_deref()).await?;
    Ok((StatusCode::CREATED, Json(entry)))
}

#[tracing::instrument(skip_all, fields(action = "stop_timer"))]
async fn stop_timer(
    State(state): State<AppState>,
    UserId(uid): UserId,
) -> Result<axum::response::Response, AppError> {
    let entry = kartoteka_domain::time_entries::stop(&state.pool, &uid).await?;
    match entry {
        Some(e) => Ok(Json(e).into_response()),
        None => Ok(StatusCode::NO_CONTENT.into_response()),
    }
}

#[tracing::instrument(skip_all, fields(action = "log_time"))]
async fn log_time(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Json(req): Json<LogRequest>,
) -> Result<impl IntoResponse, AppError> {
    let entry = kartoteka_domain::time_entries::log_manual(
        &state.pool,
        &uid,
        req.item_id.as_deref(),
        &req.started_at,
        &req.ended_at,
        req.description.as_deref(),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(entry)))
}

#[tracing::instrument(skip_all, fields(action = "assign_time_entry", entry_id = %id))]
async fn assign_entry(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
    Json(req): Json<AssignRequest>,
) -> Result<impl IntoResponse, AppError> {
    let entry =
        kartoteka_domain::time_entries::assign(&state.pool, &uid, &id, &req.item_id).await?;
    Ok(Json(entry))
}

#[tracing::instrument(skip_all, fields(action = "delete_time_entry", entry_id = %id))]
async fn delete_entry(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::time_entries::delete(&state.pool, &uid, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
