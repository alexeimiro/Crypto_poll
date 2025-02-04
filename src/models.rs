use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct Poll {
    pub id: Uuid,
    pub title: String,
    pub options: Vec<String>, 
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Vote {
    pub id: Uuid,
    pub poll_id: Uuid,
    pub option_index: i32,
    pub voter_ip: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePoll {
    pub title: String,
    pub options: Vec<String>,
    pub expires_in_minutes: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VoteRequest {
    pub option_index: i32,
}