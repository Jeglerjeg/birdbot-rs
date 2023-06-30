ALTER TABLE osu_guild_channels
ALTER COLUMN score_channel type BIGINT USING COALESCE(score_channel[1], NULL);

ALTER TABLE osu_guild_channels
ALTER COLUMN map_channel type BIGINT USING COALESCE(map_channel[1], NULL);