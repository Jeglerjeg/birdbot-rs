ALTER TABLE osu_files DROP COLUMN time_cached;

DROP TRIGGER mdt_osu_files ON osu_files;