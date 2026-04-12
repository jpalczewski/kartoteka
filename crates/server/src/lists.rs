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
        .route("/lists", get(list_all).post(create))
        .route("/lists/archived", get(list_archived))
        .route("/lists/{id}", get(get_one).put(update).delete(delete_list))
        .route("/lists/{id}/sublists", get(sublists))
        .route("/lists/{id}/archive", post(toggle_archive))
        .route("/lists/{id}/pin", post(toggle_pin))
        .route("/lists/{id}/reset", post(reset))
        .route("/lists/{id}/move", post(move_list))
        .route("/lists/{id}/features", put(set_features))
}

async fn list_all(
    State(state): State<AppState>,
    UserId(uid): UserId,
) -> Result<impl IntoResponse, AppError> {
    let lists = kartoteka_domain::lists::list_all(&state.pool, &uid).await?;
    Ok(Json(lists))
}

async fn list_archived(
    State(state): State<AppState>,
    UserId(uid): UserId,
) -> Result<impl IntoResponse, AppError> {
    let lists = kartoteka_domain::lists::list_archived(&state.pool, &uid).await?;
    Ok(Json(lists))
}

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

async fn sublists(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let lists = kartoteka_domain::lists::sublists(&state.pool, &id, &uid).await?;
    Ok(Json(lists))
}

async fn create(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Json(req): Json<CreateListRequest>,
) -> Result<impl IntoResponse, AppError> {
    let list = kartoteka_domain::lists::create(&state.pool, &uid, &req).await?;
    Ok((StatusCode::CREATED, Json(list)))
}

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

async fn reset(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let deleted = kartoteka_domain::lists::reset(&state.pool, &id, &uid).await?;
    Ok(Json(serde_json::json!({ "deleted_items": deleted })))
}

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
