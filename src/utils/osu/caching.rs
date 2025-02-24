use crate::Error;
use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::models::osu_files::OsuFile;
use crate::utils::db::beatmapsets;
use crate::utils::db::{beatmaps, osu_file};
use chrono::{DateTime, Utc};
use diesel_async::AsyncPgConnection;
use rosu_v2::Osu;
use rosu_v2::prelude::BeatmapsetExtended;
use std::sync::Arc;

pub async fn cache_beatmapset(
    connection: &mut AsyncPgConnection,
    beatmapset: BeatmapsetExtended,
    to_delete: Option<Vec<i64>>,
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

    if let Some(to_delete) = to_delete {
        for id in to_delete {
            beatmaps::delete(connection, id).await?;
            osu_file::delete(connection, id).await?;
        }
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
        let to_delete = Some(check_if_deleted(
            beatmapsets::read(connection, i64::from(beatmapset.mapset_id))
                .await?
                .ok_or("Beatmapset couldn't be fetched from db in get_beatmaps")?,
            &beatmapset,
        ));
        cache_beatmapset(connection, beatmapset, to_delete).await?;
        return Ok(beatmaps::get_single(connection, i64::from(id)).await?);
    }
    let beatmapset = osu_client.beatmapset_from_map_id(id).await?;
    cache_beatmapset(connection, beatmapset, None).await?;
    Ok(beatmaps::get_single(connection, i64::from(id)).await?)
}

pub async fn get_beatmapset(
    connection: &mut AsyncPgConnection,
    osu_client: Arc<Osu>,
    id: u32,
) -> Result<(Beatmapset, Vec<(Beatmap, OsuFile)>), Error> {
    let query_beatmapset = beatmapsets::read(connection, i64::from(id)).await?;
    if let Some(query_beatmapset) = query_beatmapset {
        if check_valid_result(&query_beatmapset.0.status, query_beatmapset.0.time_cached) {
            return Ok(query_beatmapset);
        }
        let beatmapset = osu_client.beatmapset(id).await?;
        let to_delete = Some(check_if_deleted(query_beatmapset, &beatmapset));
        cache_beatmapset(connection, beatmapset, to_delete).await?;
        return Ok(beatmapsets::read(connection, i64::from(id))
            .await?
            .ok_or("Failed to fetch beatmap in get_beatmapset")?);
    }
    let beatmapset = osu_client.beatmapset(id).await?;
    cache_beatmapset(connection, beatmapset, None).await?;
    Ok(beatmapsets::read(connection, i64::from(id))
        .await?
        .ok_or("Failed to fetch beatmap in get_beatmapset")?)
}

pub async fn get_updated_beatmapset(
    connection: &mut AsyncPgConnection,
    osu_client: Arc<Osu>,
    id: u32,
) -> Result<(Beatmapset, Vec<(Beatmap, OsuFile)>), Error> {
    let query_beatmapset = beatmapsets::read(connection, i64::from(id)).await?;
    if let Some(query_beatmapset) = query_beatmapset {
        let beatmapset = osu_client.beatmapset(id).await?;
        let to_delete = Some(check_if_deleted(query_beatmapset, &beatmapset));
        cache_beatmapset(connection, beatmapset, to_delete).await?;
        return Ok(beatmapsets::read(connection, i64::from(id))
            .await?
            .ok_or("Failed to fetch beatmap in get_beatmapset")?);
    }
    let beatmapset = osu_client.beatmapset(id).await?;
    cache_beatmapset(connection, beatmapset, None).await?;
    Ok(beatmapsets::read(connection, i64::from(id))
        .await?
        .ok_or("Failed to fetch beatmap in get_beatmapset")?)
}

pub fn check_if_deleted(
    query_beatmapset: (Beatmapset, Vec<(Beatmap, OsuFile)>),
    api_beatmapset: &BeatmapsetExtended,
) -> Vec<i64> {
    let mut to_delete = Vec::new();
    if let Some(ref beatmaps) = api_beatmapset.maps {
        for beatmap in query_beatmapset.1 {
            if !beatmaps.iter().any(|x| x.map_id.eq(&(beatmap.0.id as u32))) {
                to_delete.push(beatmap.0.id);
            }
        }
    }
    to_delete
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
