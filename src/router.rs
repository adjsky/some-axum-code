use crate::error::AppError;
use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use sqlx::PgPool;

pub fn api_router(pool: PgPool) -> Router {
    Router::new().route("/", get(handler)).with_state(pool)
}

#[derive(Serialize)]
struct Note {
    note_id: String,
    note: Option<String>,
}

async fn handler(State(pool): State<PgPool>) -> Result<Json<Vec<Note>>, AppError> {
    let notes = sqlx::query_as!(Note, "SELECT note_id, note FROM notes")
        .fetch_all(&pool)
        .await?;

    Ok(Json(notes))
}
