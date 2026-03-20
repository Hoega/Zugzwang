use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use rand::rngs::OsRng;
use sqlx::PgPool;
use tower_cookies::Cookies;
use uuid::Uuid;

use crate::error::AppError;

pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("Failed to hash password: {e}")))?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let parsed = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(format!("Invalid password hash: {e}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub async fn create_session(pool: &PgPool, user_id: Uuid) -> Result<String, AppError> {
    use base64::Engine;
    use rand::RngCore;

    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);

    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);

    sqlx::query(
        "INSERT INTO sessions (id, user_id, token, expires_at, created_at) VALUES ($1, $2, $3, $4, NOW())",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(&token)
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(token)
}

pub async fn validate_session(pool: &PgPool, token: &str) -> Result<Option<Uuid>, AppError> {
    let result: Option<(Uuid,)> = sqlx::query_as(
        "SELECT user_id FROM sessions WHERE token = $1 AND expires_at > NOW()",
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|(uid,)| uid))
}

pub async fn delete_session(pool: &PgPool, token: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM sessions WHERE token = $1")
        .bind(token)
        .execute(pool)
        .await?;
    Ok(())
}

pub struct AuthUser {
    pub user_id: Uuid,
}

impl FromRequestParts<PgPool> for AuthUser {
    type Rejection = AppError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &PgPool,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let cookies = Cookies::from_request_parts(parts, state)
                .await
                .map_err(|_| AppError::Unauthorized)?;

            let token = cookies
                .get("session")
                .ok_or(AppError::Unauthorized)?
                .value()
                .to_string();

            let user_id = validate_session(state, &token)
                .await?
                .ok_or(AppError::Unauthorized)?;

            Ok(AuthUser { user_id })
        }
    }
}

pub async fn verify_ownership(
    pool: &PgPool,
    repertoire_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM repertoires WHERE id = $1 AND user_id = $2)",
    )
    .bind(repertoire_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    if !exists {
        return Err(AppError::NotFound("Repertoire not found".into()));
    }
    Ok(())
}
