use crate::{AppState, error::AppError, extractors::UserId};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch},
};
use kartoteka_domain::containers as domain;
use kartoteka_shared::types::{
    Container, ContainerProgress, CreateContainerRequest, MoveContainerRequest,
    UpdateContainerRequest,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_containers).post(create_container))
        .route(
            "/{id}",
            get(get_container)
                .put(update_container)
                .delete(delete_container),
        )
        .route("/{id}/pin", patch(toggle_pin))
        .route("/{id}/move", patch(move_container_handler))
        .route("/{id}/progress", get(get_progress))
        .route("/{id}/children", get(get_children))
}

async fn list_containers(
    State(state): State<AppState>,
    UserId(user_id): UserId,
) -> Result<Json<Vec<Container>>, AppError> {
    let containers = domain::list_all(&state.pool, &user_id).await?;
    Ok(Json(containers))
}

async fn get_container(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> Result<Json<Container>, AppError> {
    domain::touch_last_opened(&state.pool, &id, &user_id)
        .await
        .ok(); // best-effort
    let container = domain::get_one(&state.pool, &id, &user_id).await?;
    Ok(Json(container))
}

async fn create_container(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Json(req): Json<CreateContainerRequest>,
) -> Result<(StatusCode, Json<Container>), AppError> {
    let container = domain::create(&state.pool, &user_id, &req).await?;
    Ok((StatusCode::CREATED, Json(container)))
}

async fn update_container(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
    Json(req): Json<UpdateContainerRequest>,
) -> Result<Json<Container>, AppError> {
    let container = domain::update(&state.pool, &id, &user_id, &req).await?;
    Ok(Json(container))
}

async fn delete_container(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    domain::delete(&state.pool, &id, &user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn toggle_pin(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> Result<Json<Container>, AppError> {
    let container = domain::toggle_pin(&state.pool, &id, &user_id).await?;
    Ok(Json(container))
}

async fn move_container_handler(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
    Json(req): Json<MoveContainerRequest>,
) -> Result<Json<Container>, AppError> {
    let container = domain::move_container(&state.pool, &id, &user_id, &req).await?;
    Ok(Json(container))
}

async fn get_progress(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> Result<Json<ContainerProgress>, AppError> {
    let progress = domain::get_progress(&state.pool, &id, &user_id).await?;
    Ok(Json(progress))
}

async fn get_children(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> Result<Json<Vec<Container>>, AppError> {
    let children = domain::get_children(&state.pool, &id, &user_id).await?;
    Ok(Json(children))
}
