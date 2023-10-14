use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::{Beatmapset, NewBeatmapset};
use crate::models::osu_files::OsuFile;
use crate::schema::{beatmapsets, osu_files};
use crate::Error;
use diesel::dsl::count;
use diesel::prelude::{BelongingToDsl, ExpressionMethods, QueryDsl, QueryResult};
use diesel::{insert_into, SelectableHelper};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use rosu_v2::prelude::BeatmapsetExtended;

impl From<BeatmapsetExtended> for NewBeatmapset {
    fn from(beatmapset: BeatmapsetExtended) -> NewBeatmapset {
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
}

pub async fn create(
    db: &mut AsyncPgConnection,
    beatmapset: BeatmapsetExtended,
) -> Result<(), Error> {
    let item = NewBeatmapset::from(beatmapset);

    insert_into(beatmapsets::table)
        .values(&item)
        .on_conflict(beatmapsets::id)
        .do_update()
        .set(&item)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn count_entries(db: &mut AsyncPgConnection) -> Result<i64, Error> {
    Ok(beatmapsets::table
        .select(count(beatmapsets::id))
        .get_result(db)
        .await?)
}

pub async fn read(
    db: &mut AsyncPgConnection,
    param_id: i64,
) -> Result<Option<(Beatmapset, Vec<(Beatmap, OsuFile)>)>, Error> {
    let beatmapset = beatmapsets::table
        .filter(beatmapsets::id.eq(param_id))
        .first::<Beatmapset>(db)
        .await;

    if let Ok(beatmapset) = beatmapset {
        let beatmaps = Beatmap::belonging_to(&beatmapset)
            .inner_join(osu_files::table)
            .select((Beatmap::as_select(), OsuFile::as_select()))
            .load(db)
            .await?;
        return Ok(Some((beatmapset, beatmaps)));
    }
    Ok(None)
}

pub async fn update(
    db: &mut AsyncPgConnection,
    param_id: i64,
    beatmapset: BeatmapsetExtended,
) -> QueryResult<usize> {
    diesel::update(beatmapsets::table.find(param_id))
        .set(NewBeatmapset::from(beatmapset))
        .execute(db)
        .await
}
