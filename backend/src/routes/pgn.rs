use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::services::tree;

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/api/pgn/{id}/import", post(import_pgn))
        .route("/api/pgn/{id}/export", get(export_pgn))
}

#[derive(serde::Deserialize)]
pub struct PgnImport {
    pub pgn: String,
}

#[derive(serde::Serialize)]
pub struct PgnExport {
    pub pgn: String,
}

async fn import_pgn(
    State(pool): State<PgPool>,
    Path(repertoire_id): Path<Uuid>,
    Json(input): Json<PgnImport>,
) -> Result<Json<serde_json::Value>, AppError> {
    let count = crate::services::pgn::import_pgn(&pool, repertoire_id, &input.pgn).await?;
    Ok(Json(serde_json::json!({ "imported_moves": count })))
}

async fn export_pgn(
    State(pool): State<PgPool>,
    Path(repertoire_id): Path<Uuid>,
) -> Result<Json<PgnExport>, AppError> {
    let flat = tree::get_flat_moves(&pool, repertoire_id).await?;
    let tree = tree::build_tree(flat);
    let pgn = crate::services::pgn::export_pgn(&tree);
    Ok(Json(PgnExport { pgn }))
}
