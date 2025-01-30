CREATE TABLE polls (
    id UUID PRIMARY KEY,
    title TEXT NOT NULL,
    options JSONB NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE votes (
    id UUID PRIMARY KEY,
    poll_id UUID REFERENCES polls(id) ON DELETE CASCADE,
    option_index INT NOT NULL
);