-- Remove the redundant idx_prt_token_hash index.
-- The UNIQUE constraint on token_hash already creates a unique index,
-- so the explicit CREATE INDEX creates a duplicate.

DROP INDEX IF EXISTS idx_prt_token_hash;
