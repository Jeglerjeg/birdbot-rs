DROP INDEX "messages_text_fts_idx";

DROP EXTENSION pg_trgm;

ALTER TABLE summary_messages ADD COLUMN ts tsvector NOT NULL
    GENERATED ALWAYS AS (to_tsvector('simple', content)) STORED;

CREATE INDEX textsearch_content_index ON summary_messages USING GIN (ts)
