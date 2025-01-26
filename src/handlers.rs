// src/handlers.rs
use warp::Reply;
use sqlx::PgPool;
use crate::poll::{get_votes, vote};

pub async fn list_cryptos(pool: PgPool) -> Result<impl Reply, warp::Rejection> {
    let votes = get_votes(&pool).await.unwrap_or_default();
    Ok(warp::reply::json(&votes))
}

pub async fn vote_for_crypto(symbol: String, pool: PgPool) -> Result<impl Reply, warp::Rejection> {
    let response = vote(&pool, symbol).await.unwrap_or_else(|_| "Failed to record vote".to_string());
    Ok(warp::reply::json(&response))
}