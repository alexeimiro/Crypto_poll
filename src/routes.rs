use axum::{Router, routing::{get, post}};
use crate::handlers;

pub fn create_router() -> Router<sqlx::PgPool> {
    Router::new()
        .route("/api/polls", post(handlers::create_poll))
        .route("/api/polls/current", get(handlers::get_current_poll))
        .route("/api/votes", post(handlers::submit_vote))
        .route("/api/results", get(handlers::get_results))
}