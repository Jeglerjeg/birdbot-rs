DROP INDEX textsearch_content_index;

ALTER TABLE summary_messages DROP COLUMN ts;

ALTER TABLE summary_messages ALTER COLUMN content TYPE text;

CREATE EXTENSION pg_trgm;

CREATE INDEX IF NOT EXISTS "messages_text_fts_idx" ON "summary_messages" USING gin ("content" gin_trgm_ops);