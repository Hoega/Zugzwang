use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::repertoire::{CreateRepertoire, Repertoire, UpdateRepertoire};

#[derive(serde::Serialize, sqlx::FromRow)]
struct RepertoireWithCounts {
    id: uuid::Uuid,
    name: String,
    color: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    move_count: i64,
    line_count: i64,
    first_moves: Option<String>,
}

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/api/repertoires", get(list).post(create))
        .route(
            "/api/repertoires/{id}",
            get(get_one).put(update).delete(delete),
        )
}

async fn list(State(pool): State<PgPool>) -> Result<Json<Vec<RepertoireWithCounts>>, AppError> {
    let repertoires = sqlx::query_as::<_, RepertoireWithCounts>(
        r#"SELECT r.id, r.name, r.color, r.created_at, r.updated_at,
                  COUNT(m.id) AS move_count,
                  COUNT(m.id) FILTER (WHERE NOT EXISTS (
                      SELECT 1 FROM moves c WHERE c.parent_id = m.id
                  )) AS line_count,
                  (SELECT STRING_AGG(DISTINCT m2.san_move, ',' ORDER BY m2.san_move)
                   FROM moves m2 WHERE m2.repertoire_id = r.id AND m2.parent_id IS NULL
                  ) AS first_moves
           FROM repertoires r
           LEFT JOIN moves m ON m.repertoire_id = r.id
           GROUP BY r.id
           ORDER BY r.created_at DESC"#,
    )
    .fetch_all(&pool)
    .await?;
    Ok(Json(repertoires))
}

async fn create(
    State(pool): State<PgPool>,
    Json(input): Json<CreateRepertoire>,
) -> Result<Json<Repertoire>, AppError> {
    if input.color != "white" && input.color != "black" {
        return Err(AppError::Validation("color must be 'white' or 'black'".into()));
    }
    let rep = sqlx::query_as::<_, Repertoire>(
        r#"INSERT INTO repertoires (id, name, color, created_at, updated_at)
           VALUES ($1, $2, $3, NOW(), NOW())
           RETURNING *"#,
    )
    .bind(Uuid::new_v4())
    .bind(&input.name)
    .bind(&input.color)
    .fetch_one(&pool)
    .await?;
    Ok(Json(rep))
}

async fn get_one(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Repertoire>, AppError> {
    let rep = sqlx::query_as::<_, Repertoire>(
        "SELECT * FROM repertoires WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Repertoire not found".into()))?;
    Ok(Json(rep))
}

async fn update(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateRepertoire>,
) -> Result<Json<Repertoire>, AppError> {
    let rep = sqlx::query_as::<_, Repertoire>(
        r#"UPDATE repertoires SET name = COALESCE($2, name), updated_at = NOW()
           WHERE id = $1 RETURNING *"#,
    )
    .bind(id)
    .bind(&input.name)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Repertoire not found".into()))?;
    Ok(Json(rep))
}

async fn delete(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = sqlx::query("DELETE FROM repertoires WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Repertoire not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}
