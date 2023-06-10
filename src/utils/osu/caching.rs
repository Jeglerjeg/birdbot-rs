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
        for beatmap in beatmaps {
            beatmaps::create(connection, beatmap).await?;
        }
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
        for beatmap in beatmaps {
            beatmaps::create(connection, beatmap).await?;
        }
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
) -> Result<Beatmap, Error> {
    let query_beatmap = beatmaps::get_single(connection, i64::from(id)).await;
    if let Ok(beatmap) = query_beatmap {
        if check_beatmap_valid_result(&beatmap) {
            return Ok(beatmap);
        }
        update_cache(connection, osu_client, beatmap.beatmapset_id).await?;
        return Ok(beatmaps::get_single(connection, i64::from(id)).await?);
    }
    cache_beatmapset_from_beatmap(connection, osu_client, i64::from(id)).await?;
    Ok(beatmaps::get_single(connection, i64::from(id)).await?)
}

pub async fn get_beatmapset(
    connection: &mut AsyncPgConnection,
    osu_client: Arc<Osu>,
    id: u32,
) -> Result<Beatmapset, Error> {
    let query_beatmapset = beatmapsets::read(connection, i64::from(id)).await;
    if let Ok(beatmapset) = query_beatmapset {
        if check_beatmapset_valid_result(&beatmapset) {
            return Ok(beatmapset);
        }
        update_cache(connection, osu_client, beatmapset.id).await?;
        return Ok(beatmapsets::read(connection, i64::from(id)).await?);
    }
    cache_beatmapset(connection, osu_client, i64::from(id)).await?;
    Ok(beatmapsets::read(connection, i64::from(id)).await?)
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
