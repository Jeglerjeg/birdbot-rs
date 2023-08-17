CREATE TABLE IF NOT EXISTS summary_enabled_guilds (
    id BIGSERIAL PRIMARY KEY,
    guild_id BIGINT NOT NULL,
    channel_ids BIGINT[] NOT NULL
);
