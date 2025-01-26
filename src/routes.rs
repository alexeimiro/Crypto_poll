// src/routes.rs
use warp::Filter;
use sqlx::PgPool;
use crate::handlers::{list_cryptos, vote_for_crypto};

pub fn create_routes(pool: PgPool) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let list_route = warp::path("cryptos")
        .and(warp::get())
        .and(with_pool(pool.clone()))
        .and_then(list_cryptos);

    let vote_route = warp::path("vote")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_pool(pool.clone()))
        .and_then(vote_for_crypto);

    list_route.or(vote_route)
}

fn with_pool(pool: PgPool) -> impl Filter<Extract = (PgPool,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || pool.clone())
}