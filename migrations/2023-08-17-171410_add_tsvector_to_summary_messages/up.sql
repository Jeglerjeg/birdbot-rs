ALTER TABLE summary_messages ADD COLUMN ts tsvector NOT NULL
    GENERATED ALWAYS AS (to_tsvector('english', content)) STORED;

CREATE INDEX textsearch_content_index ON summary_messages USING GIN (ts)