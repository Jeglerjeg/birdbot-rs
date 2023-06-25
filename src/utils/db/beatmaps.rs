use crate::models::beatmaps::{Beatmap, NewBeatmap};
use crate::models::beatmapsets::Beatmapset;
use crate::schema::beatmaps;
use crate::schema::beatmapsets;
use crate::Error;
use diesel::insert_into;
use diesel::prelude::{ExpressionMethods, QueryDsl};
use diesel_async::{AsyncPgConnection, RunQueryDsl};

fn to_insert_beatmap(beatmap: &rosu_v2::prelude::Beatmap, osu_file: Vec<u8>) -> NewBeatmap {
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
        osu_file,
    }
}

pub async fn create(
    db: &mut AsyncPgConnection,
    beatmaps: Vec<&rosu_v2::prelude::Beatmap>,
) -> Result<(), Error> {
    let mut items = Vec::new();

    for beatmap in beatmaps {
        let response = reqwest::get(format!("https://osu.ppy.sh/osu/{}", beatmap.map_id))
            .await?
            .bytes()
            .await?;
        let osu_file = response.to_vec();
        items.push(to_insert_beatmap(beatmap, osu_file));
    }

    insert_into(beatmaps::table)
        .values(items)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn get_single(
    db: &mut AsyncPgConnection,
    param_id: i64,
) -> Result<(Beatmap, Beatmapset), diesel::result::Error> {
    beatmaps::table
        .inner_join(beatmapsets::table)
        .filter(beatmaps::id.eq(param_id))
        .first::<(Beatmap, Beatmapset)>(db)
        .await
}

pub async fn update(
    db: &mut AsyncPgConnection,
    param_id: i64,
    beatmap: &rosu_v2::prelude::Beatmap,
) -> Result<(), Error> {
    let response = reqwest::get(format!("https://osu.ppy.sh/osu/{param_id}"))
        .await?
        .bytes()
        .await?;
    let osu_file = response.to_vec();
    let item = to_insert_beatmap(beatmap, osu_file);

    diesel::update(beatmaps::table.find(param_id))
        .set(item)
        .execute(db)
        .await?;

    Ok(())
}
