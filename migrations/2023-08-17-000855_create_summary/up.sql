CREATE TABLE IF NOT EXISTS summary_messages (
    id BIGSERIAL PRIMARY KEY,
    content CHAR(4000) NOT NULL,
    discord_id BIGINT NOT NULL,
    author_id BIGINT NOT NULL,
    channel_id BIGINT NOT NULL,
    is_bot BOOLEAN NOT NULL
);

CREATE INDEX author_id_index ON summary_messages (author_id);
CREATE INDEX channel_id_index ON summary_messages (channel_id);
CREATE INDEX is_bot_index ON summary_messages (is_bot);