// src/main.rs
use axum::http::HeaderValue;
use axum::Router; // Keep this if Router is actually used
use axum_server::Server;
use dotenvy::dotenv;
use std::net::SocketAddr;
use tower_http::cors::{AllowOrigin, CorsLayer};
mod db;
mod handlers;
mod models;
mod routes;

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init(); // Initialize tracing for logging

    let pool = db::create_pool().await.expect("Failed to create pool");

    // Run migrations first
    println!("Starting database migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
    println!("Migrations completed successfully!");

    let cors_origin = std::env::var("CORS_ORIGIN")
        .expect("CORS_ORIGIN must be set")
        .parse::<HeaderValue>()
        .expect("Invalid CORS_ORIGIN format");

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(cors_origin))
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    let app = routes::create_router().with_state(pool).layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Listening on {}", addr);

    Server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}