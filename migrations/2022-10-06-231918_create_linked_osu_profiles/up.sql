CREATE TABLE IF NOT EXISTS linked_osu_profiles (
    id BIGINT NOT NULL PRIMARY KEY,
    osu_id BIGINT NOT NULL,
    home_guild BIGINT NOT NULL,
    mode TEXT NOT NULL DEFAULT "osu"
)