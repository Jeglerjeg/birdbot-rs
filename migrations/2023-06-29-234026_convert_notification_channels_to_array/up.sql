ALTER TABLE osu_guild_channels
ALTER COLUMN score_channel type BIGINT[] USING ARRAY[score_channel];

ALTER TABLE osu_guild_channels
ALTER COLUMN map_channel type BIGINT[] USING ARRAY[map_channel];