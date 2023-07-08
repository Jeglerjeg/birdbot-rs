use crate::models::beatmaps::{Beatmap, NewBeatmap};
use crate::models::beatmapsets::Beatmapset;
use crate::models::osu_files::OsuFile;
use crate::schema::beatmapsets;
use crate::schema::{beatmaps, osu_files};
use crate::Error;
use diesel::insert_into;
use diesel::prelude::{ExpressionMethods, QueryDsl};
use diesel::upsert::excluded;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

impl TryFrom<&rosu_v2::prelude::Beatmap> for NewBeatmap {
    type Error = Error;

    fn try_from(beatmap: &rosu_v2::prelude::Beatmap) -> Result<Self, Self::Error> {
        Ok(NewBeatmap {
            id: i64::from(beatmap.map_id),
            ar: f64::from(beatmap.ar),
            beatmapset_id: i64::from(beatmap.mapset_id),
            checksum: beatmap.checksum.clone(),
            max_combo: i32::try_from(beatmap.max_combo.unwrap_or(0))?,
            bpm: f64::from(beatmap.bpm),
            convert: beatmap.convert,
            count_circles: i32::try_from(beatmap.count_circles)?,
            count_sliders: i32::try_from(beatmap.count_sliders)?,
            count_spinners: i32::try_from(beatmap.count_spinners)?,
            cs: f64::from(beatmap.cs),
            difficulty_rating: f64::from(beatmap.stars),
            drain: i32::try_from(beatmap.seconds_drain)?,
            mode: beatmap.mode.to_string(),
            passcount: i32::try_from(beatmap.passcount)?,
            playcount: i32::try_from(beatmap.playcount)?,
            status: crate::utils::osu::misc_format::format_rank_status(beatmap.status),
            total_length: i32::try_from(beatmap.seconds_total)?,
            user_id: i64::from(beatmap.creator_id),
            version: beatmap.version.clone(),
        })
    }
}

pub async fn create(
    db: &mut AsyncPgConnection,
    beatmaps: Vec<&rosu_v2::prelude::Beatmap>,
) -> Result<(), Error> {
    let mut items = Vec::new();

    for beatmap in beatmaps {
        items.push(NewBeatmap::try_from(beatmap)?);
    }

    insert_into(beatmaps::table)
        .values(&items)
        .on_conflict(beatmaps::id)
        .do_update()
        .set((
            beatmaps::id.eq(excluded(beatmaps::id)),
            beatmaps::ar.eq(excluded(beatmaps::ar)),
            beatmaps::beatmapset_id.eq(excluded(beatmaps::beatmapset_id)),
            beatmaps::checksum.eq(excluded(beatmaps::checksum)),
            beatmaps::max_combo.eq(excluded(beatmaps::max_combo)),
            beatmaps::bpm.eq(excluded(beatmaps::bpm)),
            beatmaps::convert.eq(excluded(beatmaps::convert)),
            beatmaps::count_circles.eq(excluded(beatmaps::count_circles)),
            beatmaps::count_sliders.eq(excluded(beatmaps::count_sliders)),
            beatmaps::count_spinners.eq(excluded(beatmaps::count_spinners)),
            beatmaps::cs.eq(excluded(beatmaps::cs)),
            beatmaps::difficulty_rating.eq(excluded(beatmaps::difficulty_rating)),
            beatmaps::drain.eq(excluded(beatmaps::drain)),
            beatmaps::mode.eq(excluded(beatmaps::mode)),
            beatmaps::passcount.eq(excluded(beatmaps::passcount)),
            beatmaps::playcount.eq(excluded(beatmaps::playcount)),
            beatmaps::status.eq(excluded(beatmaps::status)),
            beatmaps::total_length.eq(excluded(beatmaps::total_length)),
            beatmaps::user_id.eq(excluded(beatmaps::user_id)),
            beatmaps::version.eq(excluded(beatmaps::version)),
        ))
        .execute(db)
        .await?;

    Ok(())
}

pub async fn get_single(
    db: &mut AsyncPgConnection,
    param_id: i64,
) -> Result<(Beatmap, Beatmapset, OsuFile), diesel::result::Error> {
    beatmaps::table
        .inner_join(beatmapsets::table)
        .inner_join(osu_files::table)
        .filter(beatmaps::id.eq(param_id))
        .first::<(Beatmap, Beatmapset, OsuFile)>(db)
        .await
}

pub async fn update(
    db: &mut AsyncPgConnection,
    param_id: i64,
    beatmap: &rosu_v2::prelude::Beatmap,
) -> Result<(), Error> {
    let item = NewBeatmap::try_from(beatmap)?;

    diesel::update(beatmaps::table.find(param_id))
        .set(item)
        .execute(db)
        .await?;

    Ok(())
}
