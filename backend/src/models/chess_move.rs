use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct MoveNode {
    pub id: Uuid,
    pub repertoire_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub fen: String,
    pub uci_move: String,
    pub san_move: String,
    pub move_number: i32,
    pub is_white_move: bool,
    pub comment: Option<String>,
    pub nag: Option<String>,
    pub variant_name: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMove {
    pub parent_id: Option<Uuid>,
    pub fen: String,
    pub uci_move: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMove {
    pub comment: Option<String>,
    pub nag: Option<String>,
    pub variant_name: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct MoveTreeNode {
    #[serde(flatten)]
    pub node: MoveNode,
    pub children: Vec<MoveTreeNode>,
}
