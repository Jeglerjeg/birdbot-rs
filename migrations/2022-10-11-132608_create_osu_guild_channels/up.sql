CREATE TABLE IF NOT EXISTS osu_guild_channels (
  guild_id BIGINT NOT NULL PRIMARY KEY,
  score_channel BIGINT,
  map_channel BIGINT
)