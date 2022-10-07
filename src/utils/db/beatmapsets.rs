use crate::models::beatmapsets::{Beatmapset, NewBeatmapset};
use crate::schema::beatmapsets;
use diesel::prelude::*;
use diesel::{insert_into, QueryResult, RunQueryDsl};

fn to_insert_beatmapset(beatmapset: rosu_v2::prelude::Beatmapset) -> NewBeatmapset {
    NewBeatmapset {
        id: i64::from(beatmapset.mapset_id),
        artist: beatmapset.artist,
        bpm: beatmapset.bpm,
        list_cover: beatmapset.covers.list_2x,
        cover: beatmapset.covers.cover_2x,
        creator: beatmapset.creator_name.into_string(),
        play_count: i64::from(beatmapset.playcount),
        source: beatmapset.source,
        status: crate::utils::osu::misc_format::format_rank_status(beatmapset.status),
        title: beatmapset.title,
        user_id: i64::from(beatmapset.creator_id),
    }
}

pub fn create(beatmapset: rosu_v2::prelude::Beatmapset) {
    let db = &mut crate::utils::db::establish_connection::establish_connection();
    let item = to_insert_beatmapset(beatmapset);

    insert_into(beatmapsets::table)
        .values(item)
        .execute(db)
        .expect("Couldn't insert beatmap");
}

pub fn read(param_id: i64) -> QueryResult<Beatmapset> {
    let db = &mut crate::utils::db::establish_connection::establish_connection();

    beatmapsets::table
        .filter(beatmapsets::id.eq(param_id))
        .first::<Beatmapset>(db)
}

pub fn delete(param_id: i64) -> QueryResult<usize> {
    let db = &mut crate::utils::db::establish_connection::establish_connection();

    diesel::delete(beatmapsets::table.filter(beatmapsets::id.eq(param_id))).execute(db)
}