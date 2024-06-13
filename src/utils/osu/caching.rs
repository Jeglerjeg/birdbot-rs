use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::models::osu_files::OsuFile;
use crate::utils::db::beatmapsets;
use crate::utils::db::{beatmaps, osu_file};
use crate::Error;
use chrono::{DateTime, Utc};
use diesel_async::AsyncPgConnection;
use rosu_v2::prelude::BeatmapsetExtended;
use rosu_v2::Osu;
use std::sync::Arc;

pub async fn cache_beatmapset(
    connection: &mut AsyncPgConnection,
    beatmapset: BeatmapsetExtended,
) -> Result<(), Error> {
    if let Some(ref beatmaps) = beatmapset.maps {
        let mut beatmaps_to_insert = Vec::new();
        let mut beatmap_ids = Vec::new();
        let mut osu_files_to_insert = Vec::new();
        for beatmap in beatmaps {
            beatmaps_to_insert.push(beatmap);
            beatmap_ids.push(i64::from(beatmap.map_id));
        }
        let existing_osu_files = osu_file::get_files(connection, &beatmap_ids).await?;
        for id in beatmap_ids {
            if existing_osu_files.iter().any(|file| file.id == id) {
                continue;
            }
            let response = reqwest::get(format!("https://osu.ppy.sh/osu/{id}"))
                .await?
                .bytes()
                .await?;
            let osu_file = response.to_vec();
            osu_files_to_insert.push((id, osu_file));
        }
        beatmaps::create(connection, beatmaps_to_insert).await?;
        osu_file::create(connection, osu_files_to_insert).await?;
    }

    beatmapsets::create(connection, beatmapset).await?;

    Ok(())
}

pub async fn get_beatmap(
    connection: &mut AsyncPgConnection,
    osu_client: Arc<Osu>,
    id: u32,
) -> Result<(Beatmap, Beatmapset, OsuFile), Error> {
    let query_beatmap = beatmaps::get_single(connection, i64::from(id)).await;
    if let Ok(beatmap) = query_beatmap {
        if check_valid_result(&beatmap.0.status, beatmap.0.time_cached) {
            return Ok(beatmap);
        }
        let beatmapset = osu_client.beatmapset_from_map_id(id).await?;
        cache_beatmapset(connection, beatmapset).await?;
        return Ok(beatmaps::get_single(connection, i64::from(id)).await?);
    }
    let beatmapset = osu_client.beatmapset_from_map_id(id).await?;
    cache_beatmapset(connection, beatmapset).await?;
    Ok(beatmaps::get_single(connection, i64::from(id)).await?)
}

pub async fn get_beatmapset(
    connection: &mut AsyncPgConnection,
    osu_client: Arc<Osu>,
    id: u32,
) -> Result<(Beatmapset, Vec<(Beatmap, OsuFile)>), Error> {
    let query_beatmapset = beatmapsets::read(connection, i64::from(id)).await?;
    if let Some(beatmapset) = query_beatmapset {
        if check_valid_result(&beatmapset.0.status, beatmapset.0.time_cached) {
            return Ok(beatmapset);
        }
        let beatmapset = osu_client.beatmapset(id).await?;
        cache_beatmapset(connection, beatmapset).await?;
        return Ok(beatmapsets::read(connection, i64::from(id))
            .await?
            .ok_or("Failed to fetch beatmap in get_beatmapset")?);
    }
    let beatmapset = osu_client.beatmapset(id).await?;
    cache_beatmapset(connection, beatmapset).await?;
    Ok(beatmapsets::read(connection, i64::from(id))
        .await?
        .ok_or("Failed to fetch beatmap in get_beatmapset")?)
}

pub async fn get_updated_beatmapset(
    connection: &mut AsyncPgConnection,
    osu_client: Arc<Osu>,
    id: u32,
) -> Result<(Beatmapset, Vec<(Beatmap, OsuFile)>), Error> {
    let beatmapset = osu_client.beatmapset(id).await?;
    cache_beatmapset(connection, beatmapset).await?;
    Ok(beatmapsets::read(connection, i64::from(id))
        .await?
        .ok_or("Failed to fetch beatmap in get_beatmapset")?)
}

pub fn check_valid_result(status: &str, time_cached: DateTime<Utc>) -> bool {
    let current_time = Utc::now();
    match status {
        "Loved" => {
            if (current_time - time_cached).num_days() > 182 {
                return false;
            }
        }
        "Pending" | "Graveyard" | "WIP" | "Qualified" => {
            if (current_time - time_cached).num_days() > 7 {
                return false;
            }
        }
        _ => {}
    };

    true
}
