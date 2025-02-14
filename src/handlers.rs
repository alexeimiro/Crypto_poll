// src/handlers.rs
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::{Utc, Duration};
use sqlx::PgPool;
use uuid::Uuid;
use tracing::error; // For logging errors
use crate::models::{Poll, CreatePoll, VoteRequest};

/// Creates a new poll in the database.
pub async fn create_poll(
    State(pool): State<PgPool>,
    Json(payload): Json<CreatePoll>,
) -> Result<Json<Poll>, (StatusCode, String)> {
    let mut tx = pool.begin().await.map_err(|e| {
        error!("Failed to start transaction: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database transaction failed".to_string())
    })?;

    // Delete existing votes and polls to ensure only one poll exists at a time
    if let Err(e) = sqlx::query!("DELETE FROM votes").execute(&mut *tx).await {
        error!("Failed to delete votes: {}", e);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete votes".to_string()));
    }

    if let Err(e) = sqlx::query!("DELETE FROM polls").execute(&mut *tx).await {
        error!("Failed to delete polls: {}", e);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete polls".to_string()));
    }

    // Insert the new poll
    let poll = sqlx::query_as!(
        Poll,
        r#"
        INSERT INTO polls (title, options, expires_at)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
        payload.title,
        &payload.options,
        Utc::now() + Duration::minutes(payload.expires_in_minutes)
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        error!("Failed to insert poll: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create poll".to_string())
    })?;

    tx.commit().await.map_err(|e| {
        error!("Failed to commit transaction: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Transaction commit failed".to_string())
    })?;

    Ok(Json(poll))
}

/// Retrieves the most recent poll from the database.
pub async fn get_current_poll(
    State(pool): State<PgPool>,
) -> Result<Json<Option<Poll>>, (StatusCode, String)> {
    let poll = sqlx::query_as!(
        Poll,
        r#"SELECT * FROM polls ORDER BY created_at DESC LIMIT 1"#
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        error!("Failed to fetch current poll: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch current poll".to_string())
    })?;

    Ok(Json(poll))
}

/// Submits a vote for the current poll.
pub async fn submit_vote(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<VoteRequest>,
) -> Result<Json<()>, (StatusCode, String)> {
    let voter_ip = headers
        .get("x-real-ip")
        .or_else(|| headers.get("x-forwarded-for"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // Fetch the current poll
    let current_poll = sqlx::query_as!(
        Poll,
        r#"SELECT * FROM polls ORDER BY created_at DESC LIMIT 1"#
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        error!("Failed to fetch current poll: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch current poll".to_string())
    })?
    .ok_or((StatusCode::NOT_FOUND, "No active poll".to_string()))?;

    // Check if the poll has expired
    if Utc::now() > current_poll.expires_at {
        return Err((StatusCode::BAD_REQUEST, "Poll has expired".to_string()));
    }

    // Check if the user has already voted
    let existing_vote = sqlx::query!(
        r#"SELECT id FROM votes WHERE poll_id = $1 AND voter_ip = $2"#,
        current_poll.id,
        voter_ip
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        error!("Failed to check for existing vote: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to check for existing vote".to_string())
    })?;

    if existing_vote.is_some() {
        return Err((StatusCode::BAD_REQUEST, "Already voted".to_string()));
    }

    // Insert the vote
    sqlx::query!(
        r#"
        INSERT INTO votes (poll_id, option_index, voter_ip)
        VALUES ($1, $2, $3)
        "#,
        current_poll.id,
        payload.option_index,
        voter_ip
    )
    .execute(&pool)
    .await
    .map_err(|e| {
        error!("Failed to submit vote: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to submit vote".to_string())
    })?;

    Ok(Json(()))
}

/// Retrieves the results of the current poll.
pub async fn get_results(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<(i32, i64)>>, (StatusCode, String)> {
    let current_poll = sqlx::query_as!(
        Poll,
        r#"SELECT * FROM polls ORDER BY created_at DESC LIMIT 1"#
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        error!("Failed to fetch current poll: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch current poll".to_string())
    })?
    .ok_or((StatusCode::NOT_FOUND, "No active poll".to_string()))?;

    let results = sqlx::query!(
        r#"
        SELECT option_index, COUNT(*) as count
        FROM votes
        WHERE poll_id = $1
        GROUP BY option_index
        "#,
        current_poll.id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        error!("Failed to fetch results: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch results".to_string())
    })?
    .into_iter()
    .map(|r| (r.option_index, r.count.unwrap_or(0)))
    .collect();

    Ok(Json(results))
}