use axum::{extract::State, routing::{get, post}, Json, Router};
use sqlx::PgPool;
use tower_cookies::{Cookie, Cookies};
use uuid::Uuid;

use crate::auth::{self, AuthUser};
use crate::error::AppError;
use crate::models::user::User;

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", get(me))
        .route("/api/auth/change-password", post(change_password))
}

#[derive(serde::Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(serde::Deserialize)]
struct RegisterRequest {
    username: String,
    email: String,
    password: String,
}

#[derive(serde::Serialize)]
struct AuthResponse {
    user: UserInfo,
}

#[derive(serde::Serialize)]
struct UserInfo {
    id: Uuid,
    username: String,
}

fn set_session_cookie(cookies: &Cookies, token: &str) {
    let mut cookie = Cookie::new("session", token.to_string());
    cookie.set_http_only(true);
    cookie.set_same_site(tower_cookies::cookie::SameSite::Lax);
    cookie.set_path("/");
    cookie.set_max_age(tower_cookies::cookie::time::Duration::days(30));
    cookies.add(cookie);
}

fn clear_session_cookie(cookies: &Cookies) {
    let mut cookie = Cookie::new("session", "");
    cookie.set_path("/");
    cookie.set_max_age(tower_cookies::cookie::time::Duration::ZERO);
    cookies.add(cookie);
}

async fn register(
    State(pool): State<PgPool>,
    cookies: Cookies,
    Json(input): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // Validate username
    let username = input.username.trim().to_string();
    if username.len() < 3 || username.len() > 32 {
        return Err(AppError::Validation(
            "Username must be 3-32 characters".into(),
        ));
    }

    // Validate email
    let email = input.email.trim().to_lowercase();
    if !email.contains('@') || !email.contains('.') || email.len() < 5 {
        return Err(AppError::Validation("Invalid email address".into()));
    }

    // Validate password
    if input.password.len() < 8 {
        return Err(AppError::Validation(
            "Password must be at least 8 characters".into(),
        ));
    }

    // Check username uniqueness
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
            .bind(&username)
            .fetch_one(&pool)
            .await?;
    if exists {
        return Err(AppError::Validation("Username already taken".into()));
    }

    // Check email uniqueness
    let email_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
            .bind(&email)
            .fetch_one(&pool)
            .await?;
    if email_exists {
        return Err(AppError::Validation("Email already registered".into()));
    }

    let password_hash = auth::hash_password(&input.password)?;
    let user_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, created_at) VALUES ($1, $2, $3, $4, NOW())",
    )
    .bind(user_id)
    .bind(&username)
    .bind(&email)
    .bind(&password_hash)
    .execute(&pool)
    .await?;

    let token = auth::create_session(&pool, user_id).await?;
    set_session_cookie(&cookies, &token);

    Ok(Json(AuthResponse {
        user: UserInfo {
            id: user_id,
            username,
        },
    }))
}

async fn login(
    State(pool): State<PgPool>,
    cookies: Cookies,
    Json(input): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&input.username)
        .fetch_optional(&pool)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if !auth::verify_password(&input.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }

    let token = auth::create_session(&pool, user.id).await?;
    set_session_cookie(&cookies, &token);

    Ok(Json(AuthResponse {
        user: UserInfo {
            id: user.id,
            username: user.username,
        },
    }))
}

async fn logout(
    State(pool): State<PgPool>,
    cookies: Cookies,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(cookie) = cookies.get("session") {
        auth::delete_session(&pool, cookie.value()).await?;
    }
    clear_session_cookie(&cookies);
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(serde::Deserialize)]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

async fn change_password(
    State(pool): State<PgPool>,
    auth_user: AuthUser,
    Json(input): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(auth_user.user_id)
        .fetch_one(&pool)
        .await?;

    if !auth::verify_password(&input.current_password, &user.password_hash)? {
        return Err(AppError::Validation("Current password is incorrect".into()));
    }

    if input.new_password.len() < 8 {
        return Err(AppError::Validation(
            "New password must be at least 8 characters".into(),
        ));
    }

    let new_hash = auth::hash_password(&input.new_password)?;

    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(&new_hash)
        .bind(auth_user.user_id)
        .execute(&pool)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn me(
    State(pool): State<PgPool>,
    auth: AuthUser,
) -> Result<Json<AuthResponse>, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(&pool)
        .await?;

    Ok(Json(AuthResponse {
        user: UserInfo {
            id: user.id,
            username: user.username,
        },
    }))
}
