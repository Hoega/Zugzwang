use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Repertoire {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    #[serde(skip_serializing)]
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRepertoire {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRepertoire {
    pub name: Option<String>,
}
