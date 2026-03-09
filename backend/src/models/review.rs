use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReviewState {
    pub id: Uuid,
    pub move_id: Uuid,
    pub repetition_count: i32,
    pub easiness_factor: f64,
    pub interval_days: i32,
    pub next_review: DateTime<Utc>,
    pub last_reviewed: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ReviewSubmission {
    pub move_id: Uuid,
    pub response_time_ms: i64,
    pub played_uci: String,
    pub was_correct: bool,
}
