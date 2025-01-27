// handlers.rs
use actix_web::{web, HttpResponse, Responder};
use serde_json::json;
use sqlx::{PgPool, Row}; // Import the Row trait
use crate::models::{Coin, CoinSelection, VoteRequest};

/// Fetch all coins from the database
pub async fn get_coins(pool: web::Data<PgPool>) -> impl Responder {
    match sqlx::query_as::<_, Coin>("SELECT * FROM coins")
        .fetch_all(&**pool)
        .await
    {
        Ok(coins) => HttpResponse::Ok().json(coins),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

/// Select coins for the poll (admin only)
pub async fn select_coins(
    pool: web::Data<PgPool>,
    selection: web::Json<CoinSelection>,
) -> impl Responder {
    // Clear existing coins
    if let Err(e) = sqlx::query("DELETE FROM coins")
        .execute(&**pool)
        .await
    {
        return HttpResponse::InternalServerError().json(json!({ "error": e.to_string() }));
    }

    // Insert selected coins
    for symbol in &selection.symbols {
        if let Err(e) = sqlx::query("INSERT INTO coins (symbol, price) VALUES ($1, $2)")
            .bind(symbol)
            .bind("0.0") // Placeholder price, update later
            .execute(&**pool)
            .await
        {
            return HttpResponse::InternalServerError().json(json!({ "error": e.to_string() }));
        }
    }

    HttpResponse::Ok().json(json!({ "status": "Coins selected successfully" }))
}

/// Get the current poll state
pub async fn get_poll(pool: web::Data<PgPool>) -> impl Responder {
    // Fetch selected coins
    let coins = match sqlx::query_as::<_, Coin>("SELECT * FROM coins")
        .fetch_all(&**pool)
        .await
    {
        Ok(coins) => coins,
        Err(e) => return HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    };

    // Fetch votes
    let votes = match sqlx::query("SELECT coin_symbol, COUNT(*) as vote_count FROM votes GROUP BY coin_symbol")
        .fetch_all(&**pool)
        .await
    {
        Ok(votes) => votes,
        Err(e) => return HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    };

    // Convert votes to a HashMap
    let vote_counts: std::collections::HashMap<String, i64> = votes
        .into_iter()
        .map(|row| {
            let symbol: String = row.get("coin_symbol");
            let count: i64 = row.get("vote_count");
            (symbol, count)
        })
        .collect();

    HttpResponse::Ok().json(json!({
        "coins": coins,
        "votes": vote_counts
    }))
}

/// Vote for a coin
pub async fn vote(
    pool: web::Data<PgPool>,
    vote_data: web::Json<VoteRequest>,
) -> impl Responder {
    // Check if the user has already voted for this coin
    let existing_vote = sqlx::query("SELECT id FROM votes WHERE coin_symbol = $1 AND user_id = $2")
        .bind(&vote_data.coin_symbol)
        .bind(&vote_data.user_id)
        .fetch_optional(&**pool)
        .await;

    if let Ok(Some(_)) = existing_vote {
        return HttpResponse::BadRequest().json(json!({ "error": "Already voted" }));
    }

    // Record the vote
    if let Err(e) = sqlx::query("INSERT INTO votes (coin_symbol, user_id) VALUES ($1, $2)")
        .bind(&vote_data.coin_symbol)
        .bind(&vote_data.user_id)
        .execute(&**pool)
        .await
    {
        return HttpResponse::InternalServerError().json(json!({ "error": e.to_string() }));
    }

    HttpResponse::Ok().json(json!({ "status": "Vote recorded" }))
}