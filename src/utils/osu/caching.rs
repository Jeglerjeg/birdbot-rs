use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::utils::db::beatmaps;
use crate::utils::db::beatmapsets;
use crate::{Context, Error};
use chrono::Utc;
use diesel::PgConnection;

pub async fn cache_beatmapset(
    ctx: Context<'_>,
    connection: &mut PgConnection,
    id: i64,
) -> Result<(), Error> {
    let beatmapset = ctx.data().osu_client.beatmapset(id as u32).await?;
    if let Some(beatmaps) = beatmapset.maps {
        for beatmap in &beatmaps {
            beatmaps::create(connection, beatmap)?;
        }
    }

    Ok(())
}

pub async fn cache_beatmapset_from_beatmap(
    ctx: Context<'_>,
    connection: &mut PgConnection,
    id: i64,
) -> Result<(), Error> {
    let beatmapset = ctx
        .data()
        .osu_client
        .beatmapset_from_map_id(id as u32)
        .await?;

    beatmapsets::create(connection, beatmapset.clone());

    if let Some(beatmaps) = beatmapset.maps {
        for beatmap in &beatmaps {
            beatmaps::create(connection, beatmap)?;
        }
    }

    Ok(())
}

pub fn delete_cache(connection: &mut PgConnection, id: i64) -> Result<(), Error> {
    let beatmaps = beatmaps::get_mapset_maps(connection, id);
    for beatmap in &beatmaps {
        beatmaps::delete(connection, beatmap.id)?;
    }

    beatmapsets::delete(connection, id)?;

    Ok(())
}

pub async fn get_beatmap(ctx: Context<'_>, id: u32) -> Result<Beatmap, Error> {
    let connection = &mut ctx.data().db_pool.get().unwrap();
    let query_beatmap = beatmaps::get_single(connection, i64::from(id));
    if let Ok(beatmap) = query_beatmap {
        if check_beatmap_valid_result(&beatmap) {
            return Ok(beatmap);
        }
        delete_cache(connection, beatmap.beatmapset_id)?;
        cache_beatmapset(ctx, connection, beatmap.beatmapset_id).await?;
        return Ok(beatmaps::get_single(connection, i64::from(id)).unwrap());
    }
    cache_beatmapset_from_beatmap(ctx, connection, i64::from(id)).await?;
    Ok(beatmaps::get_single(connection, i64::from(id)).unwrap())
}

pub async fn get_beatmapset(ctx: Context<'_>, id: u32) -> Result<Beatmapset, Error> {
    let connection = &mut ctx.data().db_pool.get().unwrap();

    let query_beatmapset = beatmapsets::read(connection, i64::from(id));
    if let Ok(beatmapset) = query_beatmapset {
        if check_beatmapset_valid_result(&beatmapset) {
            return Ok(beatmapset);
        }
        delete_cache(connection, beatmapset.id)?;
        cache_beatmapset(ctx, connection, beatmapset.id).await?;
        return Ok(beatmapsets::read(connection, i64::from(id)).unwrap());
    }
    cache_beatmapset(ctx, connection, i64::from(id)).await?;
    Ok(beatmapsets::read(connection, i64::from(id)).unwrap())
}

pub fn check_beatmapset_valid_result(beatmapset: &Beatmapset) -> bool {
    let current_time = Utc::now().naive_utc();
    match beatmapset.status.as_str() {
        "Loved" => {
            if (current_time - beatmapset.time_cached).num_days() > 30 {
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
    let current_time = chrono::offset::Utc::now();
    match beatmap.status.as_str() {
        "Loved" => {
            if (current_time - beatmap.time_cached).num_days() > 30 {
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
