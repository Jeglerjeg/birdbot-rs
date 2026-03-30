use crate::Error;
use crate::models::beatmaps::{Beatmap, NewBeatmap};
use crate::models::beatmapsets::Beatmapset;
use crate::models::osu_files::OsuFile;
use crate::schema::beatmapsets;
use crate::schema::{beatmaps, osu_files};
use diesel::dsl::count;
use diesel::insert_into;
use diesel::prelude::{ExpressionMethods, QueryDsl};
use diesel_async::{AsyncPgConnection, RunQueryDsl};

impl TryFrom<&rosu_v2::prelude::BeatmapExtended> for NewBeatmap {
    type Error = Error;

    fn try_from(beatmap: &rosu_v2::prelude::BeatmapExtended) -> Result<Self, Self::Error> {
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

pub async fn count_entries(db: &mut AsyncPgConnection) -> Result<i64, Error> {
    Ok(beatmaps::table
        .select(count(beatmaps::id))
        .get_result(db)
        .await?)
}

pub async fn create(
    db: &mut AsyncPgConnection,
    beatmaps: Vec<&rosu_v2::prelude::BeatmapExtended>,
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
            beatmaps::id.eq(beatmaps::id),
            beatmaps::ar.eq(beatmaps::ar),
            beatmaps::beatmapset_id.eq(beatmaps::beatmapset_id),
            beatmaps::checksum.eq(beatmaps::checksum),
            beatmaps::max_combo.eq(beatmaps::max_combo),
            beatmaps::bpm.eq(beatmaps::bpm),
            beatmaps::convert.eq(beatmaps::convert),
            beatmaps::count_circles.eq(beatmaps::count_circles),
            beatmaps::count_sliders.eq(beatmaps::count_sliders),
            beatmaps::count_spinners.eq(beatmaps::count_spinners),
            beatmaps::cs.eq(beatmaps::cs),
            beatmaps::difficulty_rating.eq(beatmaps::difficulty_rating),
            beatmaps::drain.eq(beatmaps::drain),
            beatmaps::mode.eq(beatmaps::mode),
            beatmaps::passcount.eq(beatmaps::passcount),
            beatmaps::playcount.eq(beatmaps::playcount),
            beatmaps::status.eq(beatmaps::status),
            beatmaps::total_length.eq(beatmaps::total_length),
            beatmaps::user_id.eq(beatmaps::user_id),
            beatmaps::version.eq(beatmaps::version),
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

pub async fn delete(db: &mut AsyncPgConnection, param_id: i64) -> Result<(), Error> {
    diesel::delete(beatmaps::table.find(param_id))
        .execute(db)
        .await?;

    Ok(())
}
