use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BundleExport {
    pub version: u32,
    pub exported_at: DateTime<Utc>,
    pub repertoires: Vec<BundleRepertoire>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BundleRepertoire {
    pub name: String,
    pub color: String,
    pub moves: Vec<BundleMove>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BundleMove {
    pub parent_index: Option<usize>,
    pub fen: String,
    pub uci_move: String,
    pub san_move: String,
    pub move_number: i32,
    pub is_white_move: bool,
    pub comment: Option<String>,
    pub nag: Option<String>,
    pub variant_name: Option<String>,
    pub sort_order: i32,
}
