use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::chess_move::MoveNode;
use crate::models::review::{ReviewState, ReviewSubmission};
use crate::services::sm2;
use crate::services::tree;

#[derive(serde::Deserialize)]
pub struct DrillQuery {
    pub variant: Option<String>,
    pub from_move_id: Option<Uuid>,
}

#[derive(serde::Serialize)]
pub struct DrillPosition {
    pub move_node: MoveNode,
    pub line: Vec<MoveNode>,
    pub review_state: ReviewState,
}

#[derive(serde::Serialize)]
pub struct DrillBatch {
    pub positions: Vec<DrillPosition>,
    pub total_due: i64,
}

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/api/training/{id}/next", get(next_batch))
        .route("/api/training/review", post(submit_review))
}

async fn next_batch(
    State(pool): State<PgPool>,
    Path(repertoire_id): Path<Uuid>,
    Query(query): Query<DrillQuery>,
) -> Result<Json<DrillBatch>, AppError> {
    // Get repertoire color
    let rep_color: String = sqlx::query_scalar(
        "SELECT color FROM repertoires WHERE id = $1",
    )
    .bind(repertoire_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Repertoire not found".into()))?;

    let is_white = rep_color == "white";

    // If variant filter requested, get descendant move IDs
    let variant_ids: Option<Vec<Uuid>> = if let Some(ref variant_name) = query.variant {
        let labeled_move: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM moves WHERE repertoire_id = $1 AND variant_name = $2 LIMIT 1",
        )
        .bind(repertoire_id)
        .bind(variant_name)
        .fetch_optional(&pool)
        .await?;

        match labeled_move {
            Some(move_id) => Some(tree::get_descendant_ids(&pool, move_id).await?),
            None => Some(vec![]), // variant not found, return nothing
        }
    } else if let Some(from_id) = query.from_move_id {
        Some(tree::get_descendant_ids(&pool, from_id).await?)
    } else {
        None
    };

    // Get due moves
    let due_moves = if let Some(ref ids) = variant_ids {
        sqlx::query_as::<_, MoveNode>(
            r#"SELECT m.* FROM moves m
               INNER JOIN review_state rs ON rs.move_id = m.id
               WHERE m.repertoire_id = $1
                 AND m.is_white_move = $2
                 AND rs.next_review <= NOW()
                 AND m.id = ANY($3)
               ORDER BY rs.next_review ASC
               LIMIT 20"#,
        )
        .bind(repertoire_id)
        .bind(is_white)
        .bind(ids)
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query_as::<_, MoveNode>(
            r#"SELECT m.* FROM moves m
               INNER JOIN review_state rs ON rs.move_id = m.id
               WHERE m.repertoire_id = $1
                 AND m.is_white_move = $2
                 AND rs.next_review <= NOW()
               ORDER BY rs.next_review ASC
               LIMIT 20"#,
        )
        .bind(repertoire_id)
        .bind(is_white)
        .fetch_all(&pool)
        .await?
    };

    let total_due: i64 = if let Some(ref ids) = variant_ids {
        sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM moves m
               INNER JOIN review_state rs ON rs.move_id = m.id
               WHERE m.repertoire_id = $1
                 AND m.is_white_move = $2
                 AND rs.next_review <= NOW()
                 AND m.id = ANY($3)"#,
        )
        .bind(repertoire_id)
        .bind(is_white)
        .bind(ids)
        .fetch_one(&pool)
        .await
        .unwrap_or(0)
    } else {
        sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM moves m
               INNER JOIN review_state rs ON rs.move_id = m.id
               WHERE m.repertoire_id = $1
                 AND m.is_white_move = $2
                 AND rs.next_review <= NOW()"#,
        )
        .bind(repertoire_id)
        .bind(is_white)
        .fetch_one(&pool)
        .await
        .unwrap_or(0)
    };

    let mut positions = Vec::new();
    for move_node in due_moves {
        let line = tree::get_line_to_root(&pool, move_node.id).await?;
        let review_state = sqlx::query_as::<_, ReviewState>(
            "SELECT * FROM review_state WHERE move_id = $1",
        )
        .bind(move_node.id)
        .fetch_one(&pool)
        .await?;

        positions.push(DrillPosition {
            move_node,
            line,
            review_state,
        });
    }

    // Sort by line prefix so positions sharing opening moves are adjacent,
    // maximizing the benefit of skipping shared prefixes during drilling
    positions.sort_by(|a, b| {
        let a_ids: Vec<_> = a.line.iter().map(|m| m.id).collect();
        let b_ids: Vec<_> = b.line.iter().map(|m| m.id).collect();
        a_ids.cmp(&b_ids)
    });

    Ok(Json(DrillBatch {
        positions,
        total_due,
    }))
}

async fn submit_review(
    State(pool): State<PgPool>,
    Json(input): Json<ReviewSubmission>,
) -> Result<Json<ReviewState>, AppError> {
    let state = sqlx::query_as::<_, ReviewState>(
        "SELECT * FROM review_state WHERE move_id = $1",
    )
    .bind(input.move_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Review state not found".into()))?;

    let quality = sm2::grade_from_response(input.was_correct, input.response_time_ms);
    let result = sm2::calculate(&state, quality);
    let next_review = Utc::now() + chrono::Duration::days(result.interval_days as i64);

    let updated = sqlx::query_as::<_, ReviewState>(
        r#"UPDATE review_state SET
           repetition_count = $2,
           easiness_factor = $3,
           interval_days = $4,
           next_review = $5,
           last_reviewed = NOW()
           WHERE move_id = $1
           RETURNING *"#,
    )
    .bind(input.move_id)
    .bind(result.repetition_count)
    .bind(result.easiness_factor)
    .bind(result.interval_days)
    .bind(next_review)
    .fetch_one(&pool)
    .await?;

    // Log the review
    sqlx::query(
        r#"INSERT INTO review_log (id, move_id, quality, response_time_ms, played_uci, was_correct, reviewed_at)
           VALUES ($1, $2, $3, $4, $5, $6, NOW())"#,
    )
    .bind(Uuid::new_v4())
    .bind(input.move_id)
    .bind(quality)
    .bind(input.response_time_ms)
    .bind(&input.played_uci)
    .bind(input.was_correct)
    .execute(&pool)
    .await?;

    Ok(Json(updated))
}
