use axum::{
    extract::Extension,
    http::{header::HeaderValue, Method, StatusCode},
    middleware,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, types::Json as SqlxJson, FromRow, PgPool};
use tower_http::cors::{CorsLayer, AllowOrigin, AllowMethods, AllowHeaders};
use uuid::Uuid;

#[derive(Debug)]
struct AppError(sqlx::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Database error: {}", self.0) })),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError(err)
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

async fn validate_content_type(
    request: axum::extract::Request,
    next: middleware::Next,
) -> impl IntoResponse {
    let content_type = request.headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok());

    match content_type {
        Some(v) if v.contains("application/json") => next.run(request).await,
        _ => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid content type. Please use application/json" })),
        ).into_response(),
    }
}

#[axum::debug_handler]
async fn create_poll(
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<CreatePollRequest>,
) -> Result<Json<Poll>, AppError> {
    if payload.title.trim().is_empty() {
        return Err(AppError(sqlx::Error::Decode("Title cannot be empty".into())));
    }

    if payload.options.len() < 2 {
        return Err(AppError(sqlx::Error::Decode("At least 2 options required".into())));
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
        payload.title,
        SqlxJson(payload.options) as _,
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

    let total_votes = sqlx::query!(
        "SELECT COUNT(*) as count FROM votes WHERE poll_id = $1",
        poll_id
    )
    .fetch_one(&pool)
    .await?
    .count
    .unwrap_or(0);

    let mut options = Vec::new();

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
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;

    // CORS configuration
    let cors_origin = std::env::var("CORS_ORIGIN")
        .unwrap_or_else(|_| "https://crypto-poll-frontend.onrender.com".to_string());

    let cors = CorsLayer::new()
        .allow_origin(cors_origin.parse::<HeaderValue>()?)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(AllowHeaders::any())
        .allow_credentials(false);

    let app = Router::new()
        .route("/polls", post(create_poll))
        .route("/polls/{id}/results", get(get_poll_results))
        .layer(Extension(pool))
        .layer(cors)
        .layer(middleware::from_fn(validate_content_type));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("Server running on port 3000");
    axum::serve(listener, app).await?;

    Ok(())
}