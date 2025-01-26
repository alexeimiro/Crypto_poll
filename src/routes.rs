use warp::Filter;
use sqlx::PgPool;
use std::env;
use crate::handlers::{list_cryptos, vote_for_crypto};

pub fn create_routes(pool: PgPool) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // Load the frontend URL from the environment variable
    let frontend_url = env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Define CORS configuration
    let cors = warp::cors()
        .allow_origin(frontend_url.as_str()) // Use the frontend URL from the environment variable
        .allow_methods(vec!["GET", "POST"]) // Allow GET and POST requests
        .allow_headers(vec!["Content-Type"]); // Allow Content-Type header

    // Define the list_cryptos route with CORS
    let list_route = warp::path("cryptos")
        .and(warp::get())
        .and(with_pool(pool.clone()))
        .and_then(list_cryptos)
        .with(cors.clone()); // Apply CORS to this route

    // Define the vote_for_crypto route with CORS
    let vote_route = warp::path("vote")
        .and(warp::post())
        .and(warp::body::bytes()) // Accept raw bytes (plain text)
        .and(with_pool(pool.clone()))
        .and_then(vote_for_crypto)
        .with(cors.clone()); // Apply CORS to this route

    // Combine the routes
    list_route.or(vote_route)
}

fn with_pool(pool: PgPool) -> impl Filter<Extract = (PgPool,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || pool.clone())
}