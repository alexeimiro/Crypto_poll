use axum::{
    extract::Extension,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, types::Json as SqlxJson, FromRow, PgPool};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug)]
struct AppError(sqlx::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", self.0),
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

#[axum::debug_handler]
async fn create_poll(
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<CreatePollRequest>,
) -> Result<Json<Poll>, AppError> {
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
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;

    let app = Router::new()
        .route("/polls", post(create_poll))
        .route("/polls/{id}/results", get(get_poll_results))
        .layer(Extension(pool));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}