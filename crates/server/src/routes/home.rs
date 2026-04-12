use axum::{extract::State, routing::get, Json, Router};
use kartoteka_domain::home as domain;
use kartoteka_shared::types::HomeData;
use crate::{error::AppError, extractors::UserId, AppState};

pub fn routes() -> Router<AppState> {
    Router::new().route("/home", get(home_handler))
}

async fn home_handler(
    State(state): State<AppState>,
    UserId(user_id): UserId,
) -> Result<Json<HomeData>, AppError> {
    let data = domain::query(&state.pool, &user_id).await?;
    Ok(Json(data))
}
