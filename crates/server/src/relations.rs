use crate::{AppError, AppState, UserId};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get},
};

#[derive(serde::Deserialize)]
struct EntityQuery {
    entity_type: String,
    entity_id: String,
}

#[derive(serde::Deserialize)]
pub struct CreateRelationRequest {
    pub from_type: String,
    pub from_id: String,
    pub to_type: String,
    pub to_id: String,
    pub relation_type: String,
}

pub fn relations_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_for_entity).post(create_relation))
        .route("/{id}", delete(delete_relation))
}

#[tracing::instrument(skip_all, fields(action = "list_relations", entity_type = %q.entity_type, entity_id = %q.entity_id))]
async fn list_for_entity(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Query(q): Query<EntityQuery>,
) -> Result<impl IntoResponse, AppError> {
    let rels = kartoteka_domain::relations::get_for_entity(
        &state.pool,
        &uid,
        &q.entity_type,
        &q.entity_id,
    )
    .await?;
    Ok(Json(rels))
}

#[tracing::instrument(skip_all, fields(action = "create_relation"))]
async fn create_relation(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Json(req): Json<CreateRelationRequest>,
) -> Result<impl IntoResponse, AppError> {
    let rel = kartoteka_domain::relations::create(
        &state.pool,
        &uid,
        &req.from_type,
        &req.from_id,
        &req.to_type,
        &req.to_id,
        &req.relation_type,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(rel)))
}

#[tracing::instrument(skip_all, fields(action = "delete_relation", relation_id = %id))]
async fn delete_relation(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    kartoteka_domain::relations::delete(&state.pool, &uid, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
