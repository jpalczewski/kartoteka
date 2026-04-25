use crate::{AppError, AppState, UserId};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};

#[derive(serde::Deserialize)]
struct CreateFromListRequest {
    list_id: String,
    name: String,
}

#[derive(serde::Deserialize)]
struct CreateListFromTemplateRequest {
    list_name: String,
    #[serde(default = "default_list_type")]
    list_type: String,
}

fn default_list_type() -> String {
    "checklist".to_string()
}

pub fn templates_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_templates))
        .route("/from-list", post(create_from_list))
        .route("/{id}", get(get_template).delete(delete_template))
        .route("/{id}/create-list", post(create_list_from_template))
}

#[tracing::instrument(skip_all, fields(action = "list_templates"))]
async fn list_templates(
    State(state): State<AppState>,
    UserId(uid): UserId,
) -> Result<impl IntoResponse, AppError> {
    let templates = kartoteka_domain::templates::list(&state.pool, &uid).await?;
    Ok(Json(templates))
}

#[tracing::instrument(skip_all, fields(action = "create_from_list"))]
async fn create_from_list(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Json(req): Json<CreateFromListRequest>,
) -> Result<impl IntoResponse, AppError> {
    let tmpl =
        kartoteka_domain::templates::create_from_list(&state.pool, &uid, &req.list_id, &req.name)
            .await?;
    Ok((StatusCode::CREATED, Json(tmpl)))
}

#[tracing::instrument(skip_all, fields(action = "get_template", template_id = %id))]
async fn get_template(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let tmpl = kartoteka_domain::templates::get(&state.pool, &uid, &id).await?;
    tmpl.map(Json).ok_or(AppError::NotFound("template"))
}

#[tracing::instrument(skip_all, fields(action = "delete_template", template_id = %id))]
async fn delete_template(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let deleted = kartoteka_domain::templates::delete(&state.pool, &uid, &id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT.into_response())
    } else {
        Err(AppError::NotFound("template"))
    }
}

#[tracing::instrument(skip_all, fields(action = "create_list_from_template", template_id = %id))]
async fn create_list_from_template(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
    Json(req): Json<CreateListFromTemplateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let list = kartoteka_domain::templates::create_list_from_template(
        &state.pool,
        &uid,
        &id,
        &req.list_name,
        &req.list_type,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(list)))
}
