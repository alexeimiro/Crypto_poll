use warp::Reply;
use warp::hyper::body::Bytes;
use sqlx::PgPool;
use crate::poll::{get_votes, vote};

pub async fn list_cryptos(pool: PgPool) -> Result<impl Reply, warp::Rejection> {
    let votes = get_votes(&pool).await.unwrap_or_default();
    Ok(warp::reply::json(&votes))
}

pub async fn vote_for_crypto(body: Bytes, pool: PgPool) -> Result<impl Reply, warp::Rejection> {
    // Convert the request body (plain text) to a string
    let symbol = String::from_utf8(body.to_vec()).map_err(|_| warp::reject())?;

    // Call the vote function with the symbol
    let response = vote(&pool, symbol).await.unwrap_or_else(|_| "Failed to record vote".to_string());

    // Return the response as JSON
    Ok(warp::reply::json(&response))
}