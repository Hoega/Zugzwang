use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::bundle::{BundleExport, BundleMove, BundleRepertoire};
use crate::models::chess_move::MoveNode;
use crate::models::repertoire::Repertoire;

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/api/bundle/export", get(export_bundle))
        .route("/api/bundle/import", post(import_bundle))
}

async fn export_bundle(
    State(pool): State<PgPool>,
    auth: AuthUser,
) -> Result<Json<BundleExport>, AppError> {
    let repertoires = sqlx::query_as::<_, Repertoire>(
        "SELECT * FROM repertoires WHERE user_id = $1 ORDER BY created_at",
    )
    .bind(auth.user_id)
    .fetch_all(&pool)
    .await?;

    let mut bundle_reps = Vec::new();

    for rep in &repertoires {
        // Fetch all moves ordered by created_at for stable parent_index mapping
        let moves = sqlx::query_as::<_, MoveNode>(
            "SELECT * FROM moves WHERE repertoire_id = $1 ORDER BY created_at",
        )
        .bind(rep.id)
        .fetch_all(&pool)
        .await?;

        // Build UUID -> array index map
        let id_to_index: HashMap<Uuid, usize> = moves
            .iter()
            .enumerate()
            .map(|(i, m)| (m.id, i))
            .collect();

        let bundle_moves: Vec<BundleMove> = moves
            .iter()
            .map(|m| BundleMove {
                parent_index: m.parent_id.and_then(|pid| id_to_index.get(&pid).copied()),
                fen: m.fen.clone(),
                uci_move: m.uci_move.clone(),
                san_move: m.san_move.clone(),
                move_number: m.move_number,
                is_white_move: m.is_white_move,
                comment: m.comment.clone(),
                nag: m.nag.clone(),
                variant_name: m.variant_name.clone(),
                sort_order: m.sort_order,
            })
            .collect();

        bundle_reps.push(BundleRepertoire {
            name: rep.name.clone(),
            color: rep.color.clone(),
            moves: bundle_moves,
        });
    }

    Ok(Json(BundleExport {
        version: 1,
        exported_at: chrono::Utc::now(),
        repertoires: bundle_reps,
    }))
}

fn toposort_moves(moves: &[BundleMove], rep_name: &str) -> Result<Vec<usize>, AppError> {
    use std::collections::VecDeque;

    let n = moves.len();
    // Build children lists and in-degree counts
    let mut children: Vec<Vec<usize>> = vec![vec![]; n];
    let mut in_degree: Vec<usize> = vec![0; n];

    for (i, m) in moves.iter().enumerate() {
        if let Some(parent) = m.parent_index {
            if parent >= n {
                return Err(AppError::Validation(format!(
                    "Move at index {} references out-of-bounds parent {} in repertoire '{}'",
                    i, parent, rep_name
                )));
            }
            children[parent].push(i);
            in_degree[i] = 1;
        }
    }

    // BFS from roots
    let mut queue: VecDeque<usize> = VecDeque::new();
    for i in 0..n {
        if in_degree[i] == 0 {
            queue.push_back(i);
        }
    }

    let mut order = Vec::with_capacity(n);
    while let Some(idx) = queue.pop_front() {
        order.push(idx);
        for &child in &children[idx] {
            in_degree[child] -= 1;
            if in_degree[child] == 0 {
                queue.push_back(child);
            }
        }
    }

    if order.len() != n {
        return Err(AppError::Validation(format!(
            "Cycle detected in move tree for repertoire '{}'",
            rep_name
        )));
    }

    Ok(order)
}

async fn import_bundle(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(bundle): Json<BundleExport>,
) -> Result<Json<serde_json::Value>, AppError> {
    if bundle.version != 1 {
        return Err(AppError::Validation(format!(
            "Unsupported bundle version: {}",
            bundle.version
        )));
    }

    let mut total_repertoires = 0;
    let mut total_moves = 0;

    for rep in &bundle.repertoires {
        if rep.color != "white" && rep.color != "black" {
            return Err(AppError::Validation(format!(
                "Invalid color '{}' for repertoire '{}'",
                rep.color, rep.name
            )));
        }

        let mut tx = pool.begin().await?;

        let repertoire_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO repertoires (id, name, color, user_id, created_at, updated_at) VALUES ($1, $2, $3, $4, NOW(), NOW())",
        )
        .bind(repertoire_id)
        .bind(&rep.name)
        .bind(&rep.color)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await?;

        // Topologically sort moves so parents are inserted before children
        let sorted_indices = toposort_moves(&rep.moves, &rep.name)?;

        // Map from original array index to inserted UUID
        let mut index_to_uuid: HashMap<usize, Uuid> = HashMap::with_capacity(rep.moves.len());

        for &i in &sorted_indices {
            let bundle_move = &rep.moves[i];
            let parent_id = match bundle_move.parent_index {
                Some(idx) => Some(*index_to_uuid.get(&idx).ok_or_else(|| {
                    AppError::Validation(format!(
                        "Move at index {} references missing parent {} in repertoire '{}'",
                        i, idx, rep.name
                    ))
                })?),
                None => None,
            };

            let move_id = Uuid::new_v4();

            sqlx::query(
                r#"INSERT INTO moves (id, repertoire_id, parent_id, fen, uci_move, san_move,
                   move_number, is_white_move, comment, nag, variant_name, sort_order, created_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW())"#,
            )
            .bind(move_id)
            .bind(repertoire_id)
            .bind(parent_id)
            .bind(&bundle_move.fen)
            .bind(&bundle_move.uci_move)
            .bind(&bundle_move.san_move)
            .bind(bundle_move.move_number)
            .bind(bundle_move.is_white_move)
            .bind(&bundle_move.comment)
            .bind(&bundle_move.nag)
            .bind(&bundle_move.variant_name)
            .bind(bundle_move.sort_order)
            .execute(&mut *tx)
            .await?;

            // Create review_state for user-color moves
            let is_user_move = (rep.color == "white" && bundle_move.is_white_move)
                || (rep.color == "black" && !bundle_move.is_white_move);

            if is_user_move {
                sqlx::query(
                    r#"INSERT INTO review_state (id, move_id, repetition_count, easiness_factor,
                       interval_days, next_review, created_at)
                       VALUES ($1, $2, 0, 2.5, 0, NOW(), NOW())"#,
                )
                .bind(Uuid::new_v4())
                .bind(move_id)
                .execute(&mut *tx)
                .await?;
            }

            index_to_uuid.insert(i, move_id);
        }

        tx.commit().await?;
        total_repertoires += 1;
        total_moves += rep.moves.len();
    }

    Ok(Json(serde_json::json!({
        "imported_repertoires": total_repertoires,
        "imported_moves": total_moves
    })))
}
