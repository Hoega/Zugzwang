pub mod repertoires;
pub mod moves;
pub mod training;
pub mod pgn;
pub mod stats;

use axum::Router;
use sqlx::PgPool;

pub fn create_router(pool: PgPool) -> Router {
    Router::new()
        .merge(repertoires::router())
        .merge(moves::router())
        .merge(training::router())
        .merge(pgn::router())
        .merge(stats::router())
        .with_state(pool)
}
