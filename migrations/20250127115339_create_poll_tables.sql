-- Add migration script here
-- migrations/20231010120000_create_poll_tables.sql
-- Create coins table
CREATE TABLE coins (
    id SERIAL PRIMARY KEY,
    symbol TEXT NOT NULL UNIQUE,
    price TEXT NOT NULL
);

-- Create votes table
CREATE TABLE votes (
    id SERIAL PRIMARY KEY,
    coin_symbol TEXT NOT NULL REFERENCES coins(symbol),
    user_id TEXT NOT NULL,
    UNIQUE (coin_symbol, user_id)
);