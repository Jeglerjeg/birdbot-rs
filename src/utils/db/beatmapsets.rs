use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::{Beatmapset, NewBeatmapset};
use crate::schema::beatmapsets;
use crate::Error;
use diesel::prelude::{BelongingToDsl, ExpressionMethods, QueryDsl, QueryResult};
use diesel::{insert_into, SelectableHelper};
use diesel_async::{AsyncPgConnection, RunQueryDsl};

fn to_insert_beatmapset(beatmapset: rosu_v2::prelude::Beatmapset) -> NewBeatmapset {
    NewBeatmapset {
        id: i64::from(beatmapset.mapset_id),
        artist: beatmapset.artist,
        bpm: f64::from(beatmapset.bpm),
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

pub async fn create(
    db: &mut AsyncPgConnection,
    beatmapset: rosu_v2::prelude::Beatmapset,
) -> Result<(), Error> {
    let item = to_insert_beatmapset(beatmapset);

    insert_into(beatmapsets::table)
        .values(item)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn read(
    db: &mut AsyncPgConnection,
    param_id: i64,
) -> Result<Option<(Beatmapset, Vec<Beatmap>)>, Error> {
    let beatmapset = beatmapsets::table
        .filter(beatmapsets::id.eq(param_id))
        .first::<Beatmapset>(db)
        .await;

    if let Ok(beatmapset) = beatmapset {
        let beatmaps = Beatmap::belonging_to(&beatmapset)
            .select(Beatmap::as_select())
            .load(db)
            .await?;
        return Ok(Some((beatmapset, beatmaps)));
    }
    Ok(None)
}

pub async fn update(
    db: &mut AsyncPgConnection,
    param_id: i64,
    beatmapset: rosu_v2::prelude::Beatmapset,
) -> QueryResult<usize> {
    let item = to_insert_beatmapset(beatmapset);

    diesel::update(beatmapsets::table.find(param_id))
        .set(item)
        .execute(db)
        .await
}
