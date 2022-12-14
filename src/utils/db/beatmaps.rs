use crate::models::beatmaps::{Beatmap, NewBeatmap};
use crate::schema::beatmaps;
use crate::Error;
use diesel::prelude::*;
use diesel::{insert_into, QueryResult, RunQueryDsl};

fn to_insert_beatmap(beatmap: &rosu_v2::prelude::Beatmap) -> NewBeatmap {
    NewBeatmap {
        id: i64::from(beatmap.map_id),
        ar: f64::from(beatmap.ar),
        beatmapset_id: i64::from(beatmap.mapset_id),
        checksum: beatmap.checksum.clone(),
        max_combo: beatmap.max_combo.unwrap_or(0) as i32,
        bpm: f64::from(beatmap.bpm),
        convert: beatmap.convert,
        count_circles: beatmap.count_circles as i32,
        count_sliders: beatmap.count_sliders as i32,
        count_spinners: beatmap.count_spinners as i32,
        cs: f64::from(beatmap.cs),
        difficulty_rating: f64::from(beatmap.stars),
        drain: beatmap.seconds_drain as i32,
        mode: beatmap.mode.to_string(),
        passcount: beatmap.passcount as i32,
        playcount: beatmap.playcount as i32,
        status: crate::utils::osu::misc_format::format_rank_status(beatmap.status),
        total_length: beatmap.seconds_total as i32,
        user_id: i64::from(beatmap.creator_id),
        version: beatmap.version.clone(),
    }
}

pub fn create(db: &mut PgConnection, beatmap: &rosu_v2::prelude::Beatmap) -> Result<(), Error> {
    let item = to_insert_beatmap(beatmap);

    insert_into(beatmaps::table).values(item).execute(db)?;

    Ok(())
}

pub fn get_single(db: &mut PgConnection, param_id: i64) -> QueryResult<Beatmap> {
    beatmaps::table
        .filter(beatmaps::id.eq(param_id))
        .first::<Beatmap>(db)
}

pub fn update(
    db: &mut PgConnection,
    param_id: i64,
    beatmap: &rosu_v2::prelude::Beatmap,
) -> QueryResult<usize> {
    let item = to_insert_beatmap(beatmap);

    diesel::update(beatmaps::table.find(param_id))
        .set(item)
        .execute(db)
}
