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

    // Get the port from the environment (default to 3030 for local development)
    let port = env::var("PORT").unwrap_or_else(|_| "3030".to_string());
    let port = port.parse::<u16>().expect("PORT must be a valid number");

    // Create the database connection pool
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await.expect("Failed to connect to the database");

    // Pass the pool to the routes
    let routes = routes::create_routes(pool);

    // Start the server
    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
}