use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use std::collections::HashMap;

use crate::error::AppError;

#[derive(Clone)]
pub struct ExplorerState {
    pub client: reqwest::Client,
    pub lichess_token: Option<String>,
}

pub fn router() -> Router<ExplorerState> {
    Router::new()
        .route("/api/explorer/masters", get(masters))
        .route("/api/explorer/lichess", get(lichess))
}

async fn proxy_lichess(
    state: &ExplorerState,
    base_url: &str,
    params: &HashMap<String, String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut req = state.client.get(base_url).query(params);

    if let Some(ref token) = state.lichess_token {
        req = req.header("Authorization", format!("Bearer {token}"));
    }

    let resp = req
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Lichess API request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "Lichess API returned {status}: {body}"
        )));
    }

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to parse Lichess response: {e}")))?;

    Ok(Json(data))
}

async fn masters(
    State(state): State<ExplorerState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    proxy_lichess(&state, "https://explorer.lichess.ovh/masters", &params).await
}

async fn lichess(
    State(state): State<ExplorerState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    proxy_lichess(&state, "https://explorer.lichess.ovh/lichess", &params).await
}
