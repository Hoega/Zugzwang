use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{verify_ownership, AuthUser};
use crate::error::AppError;
use crate::models::chess_move::{CreateMove, MoveNode, MoveTreeNode, UpdateMove};
use crate::services::tree;
use crate::services::validation;

pub fn router() -> Router<PgPool> {
    Router::new()
        .route(
            "/api/repertoires/{id}/moves",
            get(get_tree).post(add_move),
        )
        .route(
            "/api/repertoires/{rep_id}/moves/{move_id}",
            axum::routing::put(update_move).delete(delete_move),
        )
        .route(
            "/api/repertoires/{id}/variants",
            get(get_variants),
        )
        .route(
            "/api/repertoires/{id}/transpositions",
            get(get_transpositions),
        )
        .route(
            "/api/repertoires/{id}/gaps",
            get(get_gaps),
        )
}

async fn get_tree(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<MoveTreeNode>>, AppError> {
    verify_ownership(&pool, id, auth.user_id).await?;
    let flat = tree::get_flat_moves(&pool, id).await?;
    let tree = tree::build_tree(flat);
    Ok(Json(tree))
}

async fn add_move(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(repertoire_id): Path<Uuid>,
    Json(input): Json<CreateMove>,
) -> Result<Json<MoveNode>, AppError> {
    verify_ownership(&pool, repertoire_id, auth.user_id).await?;

    // Validate the move using shakmaty
    let result = validation::validate_move(&input.fen, &input.uci_move)?;

    // Check for existing duplicate
    let existing = sqlx::query_as::<_, MoveNode>(
        "SELECT * FROM moves WHERE repertoire_id = $1 AND parent_id IS NOT DISTINCT FROM $2 AND uci_move = $3",
    )
    .bind(repertoire_id)
    .bind(input.parent_id)
    .bind(&input.uci_move)
    .fetch_optional(&pool)
    .await?;

    if let Some(existing) = existing {
        return Ok(Json(existing));
    }

    // Determine sort_order
    let max_sort: Option<i32> = sqlx::query_scalar(
        "SELECT MAX(sort_order) FROM moves WHERE repertoire_id = $1 AND parent_id IS NOT DISTINCT FROM $2",
    )
    .bind(repertoire_id)
    .bind(input.parent_id)
    .fetch_one(&pool)
    .await?;
    let sort_order = max_sort.unwrap_or(-1) + 1;

    let move_id = Uuid::new_v4();
    let node = sqlx::query_as::<_, MoveNode>(
        r#"INSERT INTO moves (id, repertoire_id, parent_id, fen, uci_move, san_move,
           move_number, is_white_move, sort_order, created_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
           RETURNING *"#,
    )
    .bind(move_id)
    .bind(repertoire_id)
    .bind(input.parent_id)
    .bind(&result.resulting_fen)
    .bind(&input.uci_move)
    .bind(&result.san_move)
    .bind(result.move_number)
    .bind(result.is_white_move)
    .bind(sort_order)
    .fetch_one(&pool)
    .await?;

    // Get repertoire color to determine if this move should be drilled
    let rep_color: String = sqlx::query_scalar(
        "SELECT color FROM repertoires WHERE id = $1",
    )
    .bind(repertoire_id)
    .fetch_one(&pool)
    .await?;

    let is_user_move = (rep_color == "white" && result.is_white_move)
        || (rep_color == "black" && !result.is_white_move);

    if is_user_move {
        sqlx::query(
            r#"INSERT INTO review_state (id, move_id, repetition_count, easiness_factor,
               interval_days, next_review, created_at)
               VALUES ($1, $2, 0, 2.5, 0, NOW(), NOW())"#,
        )
        .bind(Uuid::new_v4())
        .bind(move_id)
        .execute(&pool)
        .await?;
    }

    Ok(Json(node))
}

async fn update_move(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path((rep_id, move_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateMove>,
) -> Result<Json<MoveNode>, AppError> {
    verify_ownership(&pool, rep_id, auth.user_id).await?;

    // For variant_name: None means "don't change", Some("") means "clear it",
    // Some(name) means "set it"
    let variant_name = input.variant_name.as_deref().map(|v| {
        if v.is_empty() { None } else { Some(v) }
    });

    let node = sqlx::query_as::<_, MoveNode>(
        r#"UPDATE moves SET
           comment = COALESCE($2, comment),
           nag = COALESCE($3, nag),
           variant_name = CASE
               WHEN $4 THEN $5
               ELSE variant_name
           END
           WHERE id = $1 RETURNING *"#,
    )
    .bind(move_id)
    .bind(&input.comment)
    .bind(&input.nag)
    .bind(input.variant_name.is_some()) // $4: whether to update variant_name
    .bind(variant_name.flatten())       // $5: new value (NULL to clear)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Move not found".into()))?;
    Ok(Json(node))
}

async fn delete_move(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path((rep_id, move_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_ownership(&pool, rep_id, auth.user_id).await?;

    // Delete subtree using recursive CTE
    sqlx::query(
        r#"
        WITH RECURSIVE subtree AS (
            SELECT id FROM moves WHERE id = $1
            UNION ALL
            SELECT m.id FROM moves m INNER JOIN subtree s ON m.parent_id = s.id
        )
        DELETE FROM moves WHERE id IN (SELECT id FROM subtree)
        "#,
    )
    .bind(move_id)
    .execute(&pool)
    .await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

async fn get_variants(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<String>>, AppError> {
    verify_ownership(&pool, id, auth.user_id).await?;
    let names = tree::get_variant_names(&pool, id).await?;
    Ok(Json(names))
}

#[derive(serde::Serialize)]
struct TranspositionGroup {
    fen: String,
    move_ids: Vec<Uuid>,
}

async fn get_transpositions(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<TranspositionGroup>>, AppError> {
    verify_ownership(&pool, id, auth.user_id).await?;

    let rows: Vec<(String, Vec<Uuid>)> = sqlx::query_as(
        r#"SELECT fen, array_agg(id) as move_ids
           FROM moves WHERE repertoire_id = $1
           GROUP BY fen HAVING COUNT(DISTINCT parent_id) > 1"#,
    )
    .bind(id)
    .fetch_all(&pool)
    .await?;

    let groups = rows
        .into_iter()
        .map(|(fen, move_ids)| TranspositionGroup { fen, move_ids })
        .collect();
    Ok(Json(groups))
}

async fn get_gaps(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Uuid>>, AppError> {
    verify_ownership(&pool, id, auth.user_id).await?;

    let rep_color: String = sqlx::query_scalar(
        "SELECT color FROM repertoires WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Repertoire not found".into()))?;

    // Opponent moves are: black moves in a white repertoire, white moves in a black repertoire
    let opponent_is_white = rep_color == "black";

    let gap_ids: Vec<Uuid> = sqlx::query_scalar(
        r#"SELECT m.id FROM moves m
           LEFT JOIN moves child ON child.parent_id = m.id
           WHERE m.repertoire_id = $1
             AND m.is_white_move = $2
             AND child.id IS NULL"#,
    )
    .bind(id)
    .bind(opponent_is_white)
    .fetch_all(&pool)
    .await?;

    Ok(Json(gap_ids))
}
