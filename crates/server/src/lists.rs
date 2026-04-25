use crate::{AppError, AppState, UserId};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
};
use kartoteka_domain::lists::{
    CreateListRequest, MoveListRequest, SetFeaturesRequest, UpdateListRequest,
};

pub fn lists_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_all).post(create))
        .route("/archived", get(list_archived))
        .route("/{id}", get(get_one).put(update).delete(delete_list))
        .route("/{id}/sublists", get(sublists))
        .route("/{id}/archive", post(toggle_archive))
        .route("/{id}/pin", post(toggle_pin))
        .route("/{id}/reset", post(reset))
        .route("/{id}/move", post(move_list))
        .route("/{id}/features", put(set_features))
}

#[tracing::instrument(skip_all, fields(action = "list_lists"))]
async fn list_all(
    State(state): State<AppState>,
    UserId(uid): UserId,
) -> Result<impl IntoResponse, AppError> {
    let lists = kartoteka_domain::lists::list_all(&state.pool, &uid).await?;
    Ok(Json(lists))
}

#[tracing::instrument(skip_all, fields(action = "list_archived_lists"))]
async fn list_archived(
    State(state): State<AppState>,
    UserId(uid): UserId,
) -> Result<impl IntoResponse, AppError> {
    let lists = kartoteka_domain::lists::list_archived(&state.pool, &uid).await?;
    Ok(Json(lists))
}

#[tracing::instrument(skip_all, fields(action = "get_list", list_id = %id))]
async fn get_one(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::lists::get_one(&state.pool, &id, &uid)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound("list"))
}

#[tracing::instrument(skip_all, fields(action = "list_sublists", list_id = %id))]
async fn sublists(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let lists = kartoteka_domain::lists::sublists(&state.pool, &id, &uid).await?;
    Ok(Json(lists))
}

#[tracing::instrument(skip_all, fields(action = "create_list"))]
async fn create(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Json(req): Json<CreateListRequest>,
) -> Result<impl IntoResponse, AppError> {
    let list = kartoteka_domain::lists::create(&state.pool, &uid, &req).await?;
    Ok((StatusCode::CREATED, Json(list)))
}

#[tracing::instrument(skip_all, fields(action = "update_list", list_id = %id))]
async fn update(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
    Json(req): Json<UpdateListRequest>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::lists::update(&state.pool, &id, &uid, &req)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound("list"))
}

#[tracing::instrument(skip_all, fields(action = "delete_list", list_id = %id))]
async fn delete_list(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let deleted = kartoteka_domain::lists::delete(&state.pool, &id, &uid).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound("list"))
    }
}

#[tracing::instrument(skip_all, fields(action = "toggle_list_archive", list_id = %id))]
async fn toggle_archive(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::lists::toggle_archive(&state.pool, &id, &uid)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound("list"))
}

#[tracing::instrument(skip_all, fields(action = "toggle_list_pin", list_id = %id))]
async fn toggle_pin(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::lists::toggle_pin(&state.pool, &id, &uid)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound("list"))
}

#[tracing::instrument(skip_all, fields(action = "reset_list", list_id = %id))]
async fn reset(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let deleted = kartoteka_domain::lists::reset(&state.pool, &id, &uid).await?;
    Ok(Json(serde_json::json!({ "deleted_items": deleted })))
}

#[tracing::instrument(skip_all, fields(action = "move_list", list_id = %id))]
async fn move_list(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
    Json(req): Json<MoveListRequest>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::lists::move_list(&state.pool, &id, &uid, &req)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound("list"))
}

#[tracing::instrument(skip_all, fields(action = "set_list_features", list_id = %id))]
async fn set_features(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
    Json(req): Json<SetFeaturesRequest>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::lists::set_features(&state.pool, &id, &uid, &req)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound("list"))
}
