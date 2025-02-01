use axum::{
    extract::{Extension, Path},
    http::{
        header::{HeaderName, HeaderValue},
        Method, StatusCode,
    },
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{
    postgres::PgPoolOptions,
    types::Json as SqlxJson,
    FromRow, PgPool,
};
use std::net::SocketAddr; // Import SocketAddr for binding
use std::time::Duration;
use tower_http::cors::{AllowHeaders, CorsLayer, ExposeHeaders};
use tracing::{debug, warn};
use tracing_subscriber;
use uuid::Uuid;

// Define custom error types for consistent error handling
#[derive(Debug)]
enum AppError {
    Validation(String),
    Database(sqlx::Error),
    NotFound(String),
    InvalidUuid(String), // New variant for invalid UUIDs
}

// Implement `IntoResponse` for `AppError` to convert errors into HTTP responses
impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Database(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::InvalidUuid(msg) => (StatusCode::BAD_REQUEST, msg), // Handle invalid UUIDs
        };
        Json(json!({ "error": error_message })).into_response()
    }
}

// Automatically convert `sqlx::Error` into `AppError`
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

// Poll structure for database interactions
#[derive(Debug, Serialize, Deserialize, FromRow)]
struct Poll {
    id: Uuid,
    title: String,
    options: SqlxJson<Vec<String>>,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

// Request payload structure for creating a poll
#[derive(Debug, Deserialize)]
struct CreatePollRequest {
    title: String,
    options: Vec<String>,
    expires_at: DateTime<Utc>,
}

// Response structure for poll results
#[derive(Debug, Serialize)]
struct PollResults {
    options: Vec<PollOptionResult>,
    total_votes: i64,
}

// Structure for each poll option's result
#[derive(Debug, Serialize)]
struct PollOptionResult {
    text: String,
    votes: i64,
    percentage: f64,
}

// Handler to create a new poll
#[axum::debug_handler]
async fn create_poll(
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<CreatePollRequest>,
) -> Result<Json<Poll>, AppError> {
    let title = payload.title.trim();

    // Validate the poll title and options
    if title.is_empty() {
        return Err(AppError::Validation("Poll title cannot be empty".into()));
    }

    if payload.options.len() < 2 {
        return Err(AppError::Validation("At least 2 options required".into()));
    }

    let valid_options: Vec<String> = payload.options
        .into_iter()
        .filter(|opt| !opt.trim().is_empty())
        .collect();

    if valid_options.len() < 2 {
        return Err(AppError::Validation("At least 2 non-empty options required".into()));
    }

    // Insert the poll into the database
    let poll = sqlx::query_as!(
        Poll,
        r#"
        INSERT INTO polls (id, title, options, expires_at, created_at)
        VALUES ($1, $2, $3, $4, NOW())
        RETURNING 
            id, 
            title, 
            options as "options!: SqlxJson<Vec<String>>", 
            expires_at as "expires_at!: DateTime<Utc>", 
            created_at as "created_at!: DateTime<Utc>"
        "#,
        Uuid::new_v4(),
        title,
        SqlxJson(valid_options) as _,
        payload.expires_at
    )
    .fetch_one(&pool)
    .await?;

    Ok(Json(poll))
}

// Handler to fetch poll results by ID
#[axum::debug_handler]
async fn get_poll_results(
    Extension(pool): Extension<PgPool>,
    Path(poll_id): Path<Uuid>, // Extract UUID from path
) -> Result<Json<PollResults>, AppError> {
   debug!("Fetching results for poll: {}", poll_id);

   // Fetch the poll from the database
   let poll = sqlx::query_as!(
       Poll,
       r#"SELECT id, title, options as "options!: SqlxJson<Vec<String>>", expires_at as "expires_at!: DateTime<Utc>", created_at as "created_at!: DateTime<Utc>" 
       FROM polls WHERE id = $1"#,
       poll_id
   )
   .fetch_optional(&pool)
   .await?
   .ok_or_else(|| {
       warn!("Poll not found: {}", poll_id);
       AppError::NotFound(format!("Poll {} not found", poll_id))
   })?;

   // Fetch total votes for the poll
   let total_votes = sqlx::query!(
       "SELECT COUNT(*) as count FROM votes WHERE poll_id = $1",
       poll_id
   )
   .fetch_one(&pool)
   .await?
   .count
   .unwrap_or(0);

   // Fetch votes for each option and calculate percentages
   let mut options = Vec::with_capacity(poll.options.0.len());
   for (index, option_text) in poll.options.0.iter().enumerate() {
       let votes = sqlx::query!(
           "SELECT COUNT(*) as count FROM votes 
           WHERE poll_id = $1 AND option_index = $2",
           poll_id,
           index as i32
       )
       .fetch_one(&pool)
       .await?
       .count
       .unwrap_or(0);

       let percentage = if total_votes > 0 {
           (votes as f64 / total_votes as f64) * 100.0
       } else {
           0.0
       };

       options.push(PollOptionResult {
           text: option_text.clone(),
           votes,
           percentage,
       });
   }

   Ok(Json(PollResults { options, total_votes }))
}

// Fallback handler for unmatched routes (404 Not Found)
async fn handle_404() -> impl IntoResponse {
     (
         StatusCode::NOT_FOUND,
         Json(json!({ "error": "Endpoint not found" })),
     )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
   // Load environment variables from `.env`
   dotenv().ok();

   // Initialize tracing subscriber for logging/debugging
   tracing_subscriber::fmt()
       .with_env_filter("poll_backend=debug,tower_http=debug")
       .init();

   // Read database URL from environment variable and connect to the database pool
   let database_url = std::env::var("DATABASE_URL")
       .expect("DATABASE_URL must be set");
    
   let pool = PgPoolOptions::new()
       .max_connections(5)
       .connect(&database_url)
       .await?;

   // Read CORS origin from environment variable or use a default value
   let cors_origin = std::env::var("CORS_ORIGIN")
       .unwrap_or_else(|_| "https://crypto-poll-frontend.onrender.com".to_string());

   // Configure CORS settings to allow requests from the frontend origin
   let cors = CorsLayer::new()
       .allow_origin(cors_origin.parse::<HeaderValue>().expect("Invalid CORS origin"))
       .allow_methods([Method::GET, Method::POST])
       .allow_headers(AllowHeaders::list([
           HeaderName::from_static("content-type"),
           HeaderName::from_static("authorization"),
       ]))
       .expose_headers(ExposeHeaders::list([
           HeaderName::from_static("content-type"),
           HeaderName::from_static("authorization"),
       ]))
       .max_age(Duration::from_secs(86400)); // Cache CORS preflight response for 24 hours

   // Define application routes and middleware layers
   let app = Router::new()
       .route("/polls", post(create_poll))                 // Route to create a new poll
       .route("/polls/{id}/results", get(get_poll_results)) // Route to fetch poll results by ID
       .fallback(handle_404)                              // Fallback route for unmatched paths (404)
       .layer(cors)
       .layer(Extension(pool));

   // Start the Axum server on port 3000 and bind it to all interfaces (0.0.0.0)
   let addr = SocketAddr::from(([0, 0, 0, 0], 3000)); // Bind to all interfaces on port 3000

   println!("Server running on http://{}", addr);

   axum_server::bind(addr).serve(app.into_make_service()).await?;
                       // Await until server shuts down

 Ok(())
}
