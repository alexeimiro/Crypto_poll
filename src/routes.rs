// src/routes.rs
use axum::{Router, routing::{get, post}};
use crate::handlers;
use http::StatusCode; // Add this line

pub fn create_router() -> Router<sqlx::PgPool> {
    Router::new()
        .route("/api/polls", post(handlers::create_poll))
        .route("/api/polls/current", get(handlers::get_current_poll))
        .route("/api/votes", post(handlers::submit_vote))
        .route("/api/results", get(handlers::get_results))
        .fallback(get(|| async { (StatusCode::NOT_FOUND, "Route not found".to_string()) }))
}