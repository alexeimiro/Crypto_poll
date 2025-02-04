-- Add migration script here
-- migrations/XXXXXX_fix_foreign_key_constraint.up.sql
ALTER TABLE votes
DROP CONSTRAINT votes_poll_id_fkey;

ALTER TABLE votes
ADD CONSTRAINT votes_poll_id_fkey
FOREIGN KEY (poll_id) REFERENCES polls(id) ON DELETE CASCADE;