use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/api/stats/overview", get(overview))
        .route("/api/stats/{id}", get(repertoire_stats))
        .route("/api/stats/{id}/heatmap", get(heatmap))
}

#[derive(serde::Serialize)]
pub struct StatsOverview {
    pub total_moves: i64,
    pub due_today: i64,
    pub mastered: i64,
    pub accuracy_7d: f64,
}

#[derive(serde::Serialize)]
pub struct RepertoireStats {
    pub repertoire_id: Uuid,
    pub move_count: i64,
    pub due_count: i64,
    pub mastery_percentage: f64,
}

#[derive(serde::Serialize, sqlx::FromRow)]
pub struct HeatmapEntry {
    pub square: String,
    pub total: i64,
    pub correct: i64,
    pub accuracy: f64,
}

async fn overview(State(pool): State<PgPool>) -> Result<Json<StatsOverview>, AppError> {
    let total_moves: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM moves")
            .fetch_one(&pool)
            .await
            .unwrap_or(0);

    let due_today: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM review_state WHERE next_review <= NOW()",
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(0);

    let mastered: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM review_state WHERE interval_days >= 21 AND easiness_factor >= 2.5",
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(0);

    let accuracy_7d: f64 = sqlx::query_scalar(
        r#"SELECT COALESCE(
            CAST(SUM(CASE WHEN was_correct THEN 1 ELSE 0 END) AS FLOAT) /
            NULLIF(COUNT(*), 0), 0.0)
           FROM review_log WHERE reviewed_at >= NOW() - INTERVAL '7 days'"#,
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(0.0);

    Ok(Json(StatsOverview {
        total_moves,
        due_today,
        mastered,
        accuracy_7d,
    }))
}

async fn repertoire_stats(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<RepertoireStats>, AppError> {
    let move_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM moves WHERE repertoire_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap_or(0);

    let due_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM moves m
           INNER JOIN review_state rs ON rs.move_id = m.id
           WHERE m.repertoire_id = $1 AND rs.next_review <= NOW()"#,
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap_or(0);

    // Progress: each drillable move contributes min(interval_days, 21) / 21
    // so a move at 7-day interval = 33%, at 21+ days = 100% (mastered)
    let mastery_percentage: f64 = sqlx::query_scalar(
        r#"SELECT COALESCE(
            CAST(SUM(LEAST(rs.interval_days, 21)) AS FLOAT) /
            NULLIF(COUNT(*) * 21, 0), 0.0)
           FROM review_state rs
           INNER JOIN moves m ON m.id = rs.move_id
           WHERE m.repertoire_id = $1"#,
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap_or(0.0);

    Ok(Json(RepertoireStats {
        repertoire_id: id,
        move_count,
        due_count,
        mastery_percentage,
    }))
}

async fn heatmap(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<HeatmapEntry>>, AppError> {
    let entries = sqlx::query_as::<_, HeatmapEntry>(
        r#"SELECT
            SUBSTRING(m.uci_move FROM 3 FOR 2) as square,
            COUNT(*) as total,
            SUM(CASE WHEN rl.was_correct THEN 1 ELSE 0 END) as correct,
            COALESCE(
                CAST(SUM(CASE WHEN rl.was_correct THEN 1 ELSE 0 END) AS FLOAT) /
                NULLIF(COUNT(*), 0), 0.0
            ) as accuracy
           FROM review_log rl
           INNER JOIN moves m ON m.id = rl.move_id
           WHERE m.repertoire_id = $1
           GROUP BY SUBSTRING(m.uci_move FROM 3 FOR 2)"#,
    )
    .bind(id)
    .fetch_all(&pool)
    .await?;

    Ok(Json(entries))
}
