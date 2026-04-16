CREATE TRIGGER mdt_beatmaps
    BEFORE UPDATE ON beatmaps
    FOR EACH ROW
    EXECUTE PROCEDURE moddatetime (time_cached);

CREATE TRIGGER mdt_beatmapsets
    BEFORE UPDATE ON beatmapsets
    FOR EACH ROW
    EXECUTE PROCEDURE moddatetime (time_cached);
