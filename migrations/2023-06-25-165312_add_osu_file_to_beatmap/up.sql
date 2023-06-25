DELETE FROM beatmaps;
DELETE FROM beatmapsets;
ALTER TABLE beatmaps ADD osu_file BYTEA NOT NULL;