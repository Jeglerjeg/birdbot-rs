CREATE TABLE IF NOT EXISTS osu_users (
  id BIGINT NOT NULL PRIMARY KEY,
  username TEXT NOT NULL,
  avatar_url TEXT NOT NULL,
  country_code VARCHAR(2) NOT NULL,
  mode VARCHAR(7) NOT NULL,
  pp FLOAT NOT NULL,
  accuracy FLOAT NOT NULL,
  country_rank INTEGER NOT NULL,
  global_rank INTEGER NOT NULL,
  max_combo INTEGER NOT NULL,
  ranked_score BIGINT NOT NULL,
  ticks INTEGER NOT NULL,
  time_cached TIMESTAMPTZ DEFAULT now() NOT NULL
)