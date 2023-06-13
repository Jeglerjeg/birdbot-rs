use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::utils::db::beatmaps;
use crate::utils::db::beatmapsets;
use crate::Error;
use chrono::Utc;
use diesel_async::AsyncPgConnection;
use rosu_v2::Osu;
use std::sync::Arc;

pub async fn cache_beatmapset(
    connection: &mut AsyncPgConnection,
    osu_client: Arc<Osu>,
    id: i64,
) -> Result<(), Error> {
    let beatmapset = osu_client.beatmapset(id as u32).await?;
    if let Some(ref beatmaps) = beatmapset.maps {
        let mut to_insert = Vec::new();
        for beatmap in beatmaps {
            to_insert.push(beatmap);
        }
        beatmaps::create(connection, to_insert).await?;
    }

    beatmapsets::create(connection, beatmapset).await?;

    Ok(())
}

pub async fn cache_beatmapset_from_beatmap(
    connection: &mut AsyncPgConnection,
    osu_client: Arc<Osu>,
    id: i64,
) -> Result<(), Error> {
    let beatmapset = osu_client.beatmapset_from_map_id(id as u32).await?;

    if let Some(ref beatmaps) = beatmapset.maps {
        let mut to_insert = Vec::new();
        for beatmap in beatmaps {
            to_insert.push(beatmap);
        }
        beatmaps::create(connection, to_insert).await?;
    }

    beatmapsets::create(connection, beatmapset).await?;

    Ok(())
}

pub async fn update_cache(
    connection: &mut AsyncPgConnection,
    osu_client: Arc<Osu>,
    id: i64,
) -> Result<(), Error> {
    let beatmapset = osu_client.beatmapset(id as u32).await?;

    if let Some(ref beatmaps) = beatmapset.maps {
        for beatmap in beatmaps {
            beatmaps::update(connection, i64::from(beatmap.map_id), beatmap).await?;
        }
    }

    beatmapsets::update(connection, id, beatmapset).await?;

    Ok(())
}

pub async fn get_beatmap(
    connection: &mut AsyncPgConnection,
    osu_client: Arc<Osu>,
    id: u32,
) -> Result<(Beatmap, Beatmapset), Error> {
    let query_beatmap = beatmaps::get_single(connection, i64::from(id)).await;
    if let Ok(beatmap) = query_beatmap {
        if check_beatmap_valid_result(&beatmap.0) {
            return Ok(beatmap);
        }
        update_cache(connection, osu_client, beatmap.0.beatmapset_id).await?;
        return Ok(beatmaps::get_single(connection, i64::from(id)).await?);
    }
    cache_beatmapset_from_beatmap(connection, osu_client, i64::from(id)).await?;
    Ok(beatmaps::get_single(connection, i64::from(id)).await?)
}

pub async fn get_beatmapset(
    connection: &mut AsyncPgConnection,
    osu_client: Arc<Osu>,
    id: u32,
) -> Result<(Beatmapset, Vec<Beatmap>), Error> {
    let query_beatmapset = beatmapsets::read(connection, i64::from(id)).await?;
    if let Some(beatmapset) = query_beatmapset {
        if check_beatmapset_valid_result(&beatmapset.0) {
            return Ok(beatmapset);
        }
        update_cache(connection, osu_client, beatmapset.0.id).await?;
        return Ok(beatmapsets::read(connection, i64::from(id))
            .await?
            .ok_or("Failed to fetch beatmap in get_beatmapset")?);
    }
    cache_beatmapset(connection, osu_client, i64::from(id)).await?;
    Ok(beatmapsets::read(connection, i64::from(id))
        .await?
        .ok_or("Failed to fetch beatmap in get_beatmapset")?)
}

pub fn check_beatmapset_valid_result(beatmapset: &Beatmapset) -> bool {
    let current_time = Utc::now().naive_utc();
    match beatmapset.status.as_str() {
        "Loved" => {
            if (current_time - beatmapset.time_cached).num_days() > 182 {
                return false;
            }
        }
        "Pending" | "Graveyard" | "WIP" | "Qualified" => {
            if (current_time - beatmapset.time_cached).num_days() > 7 {
                return false;
            }
        }
        _ => {}
    };

    true
}

pub fn check_beatmap_valid_result(beatmap: &Beatmap) -> bool {
    let current_time = Utc::now();
    match beatmap.status.as_str() {
        "Loved" => {
            if (current_time - beatmap.time_cached).num_days() > 182 {
                return false;
            }
        }
        "Pending" | "Graveyard" | "WIP" | "Qualified" => {
            if (current_time - beatmap.time_cached).num_days() > 7 {
                return false;
            }
        }
        _ => {}
    };

    true
}
