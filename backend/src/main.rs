mod auth;
mod config;
mod db;
mod error;
mod models;
mod routes;
mod services;

use routes::explorer::ExplorerState;
use tower_cookies::CookieManagerLayer;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let config = config::Config::from_env();
    let pool = db::create_pool(&config.database_url).await;
    db::run_migrations(&pool).await;

    let explorer_state = ExplorerState {
        client: reqwest::Client::new(),
        lichess_token: config.lichess_api_token,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = routes::create_router(pool, explorer_state)
        .layer(cors)
        .layer(CookieManagerLayer::new());

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Starting server on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
