// main.rs
use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;

mod models;
mod handlers;
mod routes;
mod services;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok(); // Load environment variables from .env file

    // Read DATABASE_URL from environment
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Create a connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to the database");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // Start the Actix server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone())) // Share the database pool
            .configure(routes::config_routes)
            .wrap(actix_web::middleware::Logger::default())
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}