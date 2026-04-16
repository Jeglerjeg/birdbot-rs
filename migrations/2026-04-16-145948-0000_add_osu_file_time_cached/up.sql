ALTER TABLE osu_files ADD COLUMN time_cached TIMESTAMPTZ NOT NULL DEFAULT NOW();

CREATE TRIGGER mdt_osu_files
    BEFORE UPDATE ON osu_files
    FOR EACH ROW
    EXECUTE PROCEDURE moddatetime (time_cached);
