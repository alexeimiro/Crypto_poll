// src/binance.rs
use reqwest::Error;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CryptoPrice {
    pub symbol: String,
    pub price: String,
}

pub async fn fetch_crypto_prices() -> Result<Vec<CryptoPrice>, Error> {
    let url = "https://api.binance.com/api/v3/ticker/price";
    let response = reqwest::get(url).await?;
    let prices: Vec<CryptoPrice> = response.json().await?;
    Ok(prices)
}