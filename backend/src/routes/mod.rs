pub mod auth;
pub mod repertoires;
pub mod moves;
pub mod training;
pub mod pgn;
pub mod stats;
pub mod bundle;
pub mod explorer;

use axum::Router;
use sqlx::PgPool;

use explorer::ExplorerState;

pub fn create_router(pool: PgPool, explorer_state: ExplorerState) -> Router {
    Router::new()
        .merge(auth::router())
        .merge(repertoires::router())
        .merge(moves::router())
        .merge(training::router())
        .merge(pgn::router())
        .merge(stats::router())
        .merge(bundle::router())
        .with_state(pool)
        .merge(explorer::router().with_state(explorer_state))
}
