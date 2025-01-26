// src/main.rs
mod binance;
mod poll;
mod handlers;
mod routes;

use sqlx::PgPool;
use dotenv::dotenv;
use std::env;

#[tokio::main]
async fn main() {
    dotenv().ok(); // Load environment variables from .env file

    // Get the database URL from the environment
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Create a connection pool
    let pool = PgPool::connect(&database_url).await.expect("Failed to connect to the database");

    // Pass the pool to the routes
    let routes = routes::create_routes(pool);

    // Start the server
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}