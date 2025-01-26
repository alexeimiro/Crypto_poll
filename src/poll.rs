// src/poll.rs
use sqlx::PgPool;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PollResult {
    pub symbol: String,
    pub votes: i32,
}

pub async fn vote(pool: &PgPool, symbol: String) -> Result<String, sqlx::Error> {
    // Insert or update the vote count in the database
    sqlx::query!(
        r#"
        INSERT INTO votes (symbol, votes)
        VALUES ($1, 1)
        ON CONFLICT (symbol) DO UPDATE
        SET votes = votes.votes + 1
        "#,
        symbol
    )
    .execute(pool)
    .await?;

    Ok(format!("Vote recorded for {}!", symbol))
}

pub async fn get_votes(pool: &PgPool) -> Result<Vec<PollResult>, sqlx::Error> {
    // Fetch the top 3 most voted coins
    let top_votes = sqlx::query_as!(
        PollResult,
        r#"
        SELECT symbol, votes
        FROM votes
        ORDER BY votes DESC
        LIMIT 3
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(top_votes)
}