-- Add migration script here
-- migrations/YYYYMMDDHHMMSS_create_votes_table.sql
CREATE TABLE votes (
    id SERIAL PRIMARY KEY,
    symbol TEXT NOT NULL UNIQUE,
    votes INT NOT NULL DEFAULT 0
);