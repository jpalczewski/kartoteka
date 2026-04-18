use crate::{AppError, AppState, UserId};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use kartoteka_domain::tags::{CreateTagRequest, UpdateTagRequest};

/// CRUD + tree + merge. Mounted under `/tags`.
pub fn tags_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_all).post(create_tag))
        .route("/tree", get(list_tree))
        .route("/{id}", get(get_tag).put(update_tag).delete(delete_tag))
        .route("/{id}/merge/{target_id}", post(merge_tags))
}

/// Polymorphic tag-link operations. Mounted under `/tag-links`.
/// entity_type: "item" | "list" | "container"
pub fn tag_links_router() -> Router<AppState> {
    Router::new()
        .route("/{entity_type}/{entity_id}", get(get_entity_tags))
        .route(
            "/{entity_type}/{entity_id}/{tag_id}",
            post(assign_tag).delete(remove_tag),
        )
}

#[tracing::instrument(skip_all, fields(action = "list_tags"))]
async fn list_all(
    State(state): State<AppState>,
    UserId(uid): UserId,
) -> Result<impl IntoResponse, AppError> {
    let tags = kartoteka_domain::tags::list_all(&state.pool, &uid).await?;
    Ok(Json(tags))
}

#[tracing::instrument(skip_all, fields(action = "list_tag_tree"))]
async fn list_tree(
    State(state): State<AppState>,
    UserId(uid): UserId,
) -> Result<impl IntoResponse, AppError> {
    let tags = kartoteka_domain::tags::list_tree(&state.pool, &uid).await?;
    Ok(Json(tags))
}

#[tracing::instrument(skip_all, fields(action = "create_tag"))]
async fn create_tag(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Json(req): Json<CreateTagRequest>,
) -> Result<impl IntoResponse, AppError> {
    let tag = kartoteka_domain::tags::create(&state.pool, &uid, &req).await?;
    Ok((StatusCode::CREATED, Json(tag)))
}

#[tracing::instrument(skip_all, fields(action = "get_tag", tag_id = %id))]
async fn get_tag(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::tags::get_one(&state.pool, &id, &uid)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound("tag"))
}

#[tracing::instrument(skip_all, fields(action = "update_tag", tag_id = %id))]
async fn update_tag(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
    Json(req): Json<UpdateTagRequest>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::tags::update(&state.pool, &uid, &id, &req)
        .await?
        .map(Json)
        .ok_or(AppError::NotFound("tag"))
}

#[tracing::instrument(skip_all, fields(action = "delete_tag", tag_id = %id))]
async fn delete_tag(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    if kartoteka_domain::tags::delete(&state.pool, &uid, &id).await? {
        Ok(StatusCode::NO_CONTENT.into_response())
    } else {
        Err(AppError::NotFound("tag"))
    }
}

#[tracing::instrument(skip_all, fields(action = "merge_tags", tag_id = %id, target_id = %target_id))]
async fn merge_tags(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path((id, target_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let tag = kartoteka_domain::tags::merge(&state.pool, &uid, &id, &target_id).await?;
    Ok(Json(tag))
}

#[tracing::instrument(skip_all, fields(action = "get_entity_tags", entity_type = %entity_type, entity_id = %entity_id))]
async fn get_entity_tags(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path((entity_type, entity_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let tags = match entity_type.as_str() {
        "item" => kartoteka_domain::tags::get_for_item(&state.pool, &uid, &entity_id).await?,
        "list" => kartoteka_domain::tags::get_for_list(&state.pool, &uid, &entity_id).await?,
        "container" => {
            kartoteka_domain::tags::get_for_container(&state.pool, &uid, &entity_id).await?
        }
        _ => return Err(AppError::Validation("invalid_entity_type".to_string())),
    };
    Ok(Json(tags))
}

#[tracing::instrument(skip_all, fields(action = "assign_tag", entity_type = %entity_type, entity_id = %entity_id, tag_id = %tag_id))]
async fn assign_tag(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path((entity_type, entity_id, tag_id)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, AppError> {
    match entity_type.as_str() {
        "item" => {
            kartoteka_domain::tags::assign_to_item(&state.pool, &uid, &entity_id, &tag_id).await?
        }
        "list" => {
            kartoteka_domain::tags::assign_to_list(&state.pool, &uid, &entity_id, &tag_id).await?
        }
        "container" => {
            kartoteka_domain::tags::assign_to_container(&state.pool, &uid, &entity_id, &tag_id)
                .await?
        }
        _ => return Err(AppError::Validation("invalid_entity_type".to_string())),
    }
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[tracing::instrument(skip_all, fields(action = "remove_tag", entity_type = %entity_type, entity_id = %entity_id, tag_id = %tag_id))]
async fn remove_tag(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path((entity_type, entity_id, tag_id)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let removed = match entity_type.as_str() {
        "item" => {
            kartoteka_domain::tags::remove_from_item(&state.pool, &uid, &entity_id, &tag_id).await?
        }
        "list" => {
            kartoteka_domain::tags::remove_from_list(&state.pool, &uid, &entity_id, &tag_id).await?
        }
        "container" => {
            kartoteka_domain::tags::remove_from_container(&state.pool, &uid, &entity_id, &tag_id)
                .await?
        }
        _ => return Err(AppError::Validation("invalid_entity_type".to_string())),
    };
    if removed {
        Ok(StatusCode::NO_CONTENT.into_response())
    } else {
        Err(AppError::NotFound("tag_link"))
    }
}
