use axum::{
    extract::Extension,
    http::{
        header::{HeaderName, HeaderValue},
        Method, StatusCode,
    },
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{
    postgres::PgPoolOptions,
    types::Json as SqlxJson,
    FromRow, PgPool,
};
use tower_http::cors::{AllowHeaders, CorsLayer, ExposeHeaders};
use uuid::Uuid;
use std::time::Duration;

#[derive(Debug)]
enum AppError {
    Validation(String),
    Database(sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Database(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ),
        };

        let body = Json(json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct Poll {
    id: Uuid,
    title: String,
    options: SqlxJson<Vec<String>>,
    expires_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct CreatePollRequest {
    title: String,
    options: Vec<String>,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct PollResults {
    options: Vec<PollOptionResult>,
    total_votes: i64,
}

#[derive(Debug, Serialize)]
struct PollOptionResult {
    text: String,
    votes: i64,
    percentage: f64,
}

#[axum::debug_handler]
async fn create_poll(
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<CreatePollRequest>,
) -> Result<Json<Poll>, AppError> {
    // Validate request payload
    let title = payload.title.trim();
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

#[axum::debug_handler]
async fn get_poll_results(
    Extension(pool): Extension<PgPool>,
    axum::extract::Path(poll_id): axum::extract::Path<Uuid>,
) -> Result<Json<PollResults>, AppError> {
    
   // Fetch poll details
   let poll = sqlx::query_as!(
       Poll,
       r#"SELECT 
           id, 
           title, 
           options as "options!: SqlxJson<Vec<String>>", 
           expires_at as "expires_at!: DateTime<Utc>", 
           created_at as "created_at!: DateTime<Utc>" 
       FROM polls WHERE id = $1"#,
       poll_id
   )
   .fetch_one(&pool)
   .await?;

   // Fetch total votes
   let total_votes = sqlx::query!(
       "SELECT COUNT(*) as count FROM votes WHERE poll_id = $1",
       poll_id
   )
   .fetch_one(&pool)
   .await?
   .count
   .unwrap_or(0);

   let mut options = Vec::with_capacity(poll.options.0.len());

   // Fetch votes per option
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

   Ok(Json(PollResults {
       options,
       total_votes,
   }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    
   dotenvy::dotenv().ok();
    
   let database_url = std::env::var("DATABASE_URL")
       .expect("DATABASE_URL must be set");
    
   let pool = PgPoolOptions::new()
       .max_connections(5)
       .connect(&database_url)
       .await?;

   let cors_origin = std::env::var("CORS_ORIGIN")
       .unwrap_or_else(|_| "https://crypto-poll-frontend.onrender.com".to_string());

   // CORS configuration
   let cors = CorsLayer::new()
       .allow_origin(
           cors_origin
               .parse::<HeaderValue>()
               .expect("Invalid CORS origin")
       )
       .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
       .allow_headers(AllowHeaders::list([
           HeaderName::from_static("content-type"),
           HeaderName::from_static("authorization"),
       ]))
       .expose_headers(ExposeHeaders::list([
           HeaderName::from_static("content-type"),
           HeaderName::from_static("authorization"),
       ]))
       .max_age(Duration::from_secs(86400)); // 24-hour cache

   // Define routes
   let app = Router::new()
       .route("/polls", post(create_poll))
       .route("/polls/{id}/results", get(get_poll_results))
       .layer(cors)
       .layer(Extension(pool));

   // Start server
   let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
   println!("Server running on http://0.0.0.0:3000");
   
   axum::serve(listener, app).await?;

   Ok(())
}
