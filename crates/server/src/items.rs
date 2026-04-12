use crate::{AppError, AppState, UserId};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use kartoteka_domain::items::{CreateItemRequest, MoveItemRequest, UpdateItemRequest};

#[derive(serde::Deserialize)]
struct ByDateQuery {
    date: String,
}

#[derive(serde::Deserialize)]
struct CalendarQuery {
    year: i32,
    month: u32,
}

/// Routes mounted under `/lists/{list_id}/items` (list-scoped item operations).
/// Axum captures `{list_id}` from the nest path prefix — handlers extract it via Path<String>.
pub fn list_items_router() -> Router<AppState> {
    Router::new().route("/", get(list_for_list).post(create_item))
}

/// Routes mounted under `/items`.
/// Static paths (/by-date, /calendar) registered before dynamic (/{id}) — Axum handles priority automatically.
pub fn items_router() -> Router<AppState> {
    Router::new()
        .route("/by-date", get(by_date))
        .route("/calendar", get(calendar))
        .route(
            "/{id}",
            get(get_item).put(update_item).delete(delete_item),
        )
        .route("/{id}/toggle", post(toggle_complete))
        .route("/{id}/move", post(move_item))
}

async fn list_for_list(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(list_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let items =
        kartoteka_domain::items::list_for_list(&state.pool, &list_id, &user_id).await?;
    Ok(Json(items))
}

async fn create_item(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(list_id): Path<String>,
    Json(req): Json<CreateItemRequest>,
) -> Result<impl IntoResponse, AppError> {
    let item =
        kartoteka_domain::items::create(&state.pool, &user_id, &list_id, &req).await?;
    Ok((StatusCode::CREATED, Json(item)))
}

async fn get_item(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let item = kartoteka_domain::items::get_one(&state.pool, &id, &user_id).await?;
    item.map(Json).ok_or(AppError::NotFound("item"))
}

async fn update_item(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
    Json(req): Json<UpdateItemRequest>,
) -> Result<impl IntoResponse, AppError> {
    let item = kartoteka_domain::items::update(&state.pool, &user_id, &id, &req).await?;
    item.map(Json).ok_or(AppError::NotFound("item"))
}

async fn delete_item(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let deleted = kartoteka_domain::items::delete(&state.pool, &user_id, &id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound("item"))
    }
}

async fn toggle_complete(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let item =
        kartoteka_domain::items::toggle_complete(&state.pool, &user_id, &id).await?;
    item.map(Json).ok_or(AppError::NotFound("item"))
}

async fn move_item(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
    Json(req): Json<MoveItemRequest>,
) -> Result<impl IntoResponse, AppError> {
    let item =
        kartoteka_domain::items::move_item(&state.pool, &user_id, &id, &req).await?;
    item.map(Json).ok_or(AppError::NotFound("item"))
}

async fn by_date(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Query(q): Query<ByDateQuery>,
) -> Result<impl IntoResponse, AppError> {
    let items =
        kartoteka_domain::items::by_date(&state.pool, &user_id, &q.date).await?;
    Ok(Json(items))
}

async fn calendar(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Query(q): Query<CalendarQuery>,
) -> Result<impl IntoResponse, AppError> {
    let year_month = format!("{:04}-{:02}", q.year, q.month);
    let items =
        kartoteka_domain::items::calendar(&state.pool, &user_id, &year_month).await?;
    Ok(Json(items))
}
