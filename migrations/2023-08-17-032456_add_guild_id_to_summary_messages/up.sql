DELETE FROM summary_messages;

ALTER TABLE summary_messages ADD COLUMN guild_id BIGINT NOT NULL;

CREATE INDEX guild_id_index ON summary_messages (guild_id);