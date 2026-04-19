use crate::{AppError, AppState, UserId};
use axum::{
    Json, Router,
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
};

#[derive(serde::Deserialize)]
struct SearchQuery {
    q: String,
}

pub fn search_router() -> Router<AppState> {
    Router::new().route("/", get(search))
}

#[tracing::instrument(skip_all, fields(action = "search", query = %params.q))]
async fn search(
    State(state): State<AppState>,
    UserId(uid): UserId,
    Query(params): Query<SearchQuery>,
) -> Result<impl IntoResponse, AppError> {
    let results = kartoteka_domain::search::search(&state.pool, &uid, &params.q).await?;
    Ok(Json(results))
}
