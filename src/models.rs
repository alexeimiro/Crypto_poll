// models.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Coin {
    pub id: i32,
    pub symbol: String,
    pub price: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Vote {
    pub id: i32,
    pub coin_symbol: String,
    pub user_id: String,
}

#[derive(Deserialize)]
pub struct CoinSelection {
    pub symbols: Vec<String>,
}

#[derive(Deserialize)]
pub struct VoteRequest {
    pub coin_symbol: String,
    pub user_id: String,
}