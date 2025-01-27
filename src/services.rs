// services.rs
use serde_json::Value;
use crate::models::Coin;

pub async fn fetch_coins() -> Result<Vec<Coin>, reqwest::Error> {
    let response = reqwest::get("https://api.binance.com/api/v3/ticker/price")
        .await?
        .json::<Vec<Value>>()
        .await?;

    Ok(response
        .into_iter()
        .filter_map(|v| {
            Some(Coin {
                id: 0, // Placeholder, will be replaced by the database
                symbol: v.get("symbol")?.as_str()?.to_string(),
                price: v.get("price")?.as_str()?.to_string(),
            })
        })
        .collect())
}