use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::utils::db::beatmaps;
use crate::utils::db::beatmapsets;
use crate::{Context, Error};
use chrono::Utc;

pub async fn cache_beatmapset(ctx: Context<'_>, id: i64) -> Result<(), Error> {
    let beatmapset = ctx.data().osu_client.beatmapset(id as u32).await?;

    if let Some(beatmaps) = beatmapset.maps {
        for beatmap in &beatmaps {
            beatmaps::create(beatmap);
        }
    }

    Ok(())
}

pub async fn cache_beatmapset_from_beatmap(ctx: Context<'_>, id: i64) -> Result<(), Error> {
    let beatmapset = ctx
        .data()
        .osu_client
        .beatmapset_from_map_id(id as u32)
        .await?;

    beatmapsets::create(beatmapset.clone());

    if let Some(beatmaps) = beatmapset.maps {
        for beatmap in &beatmaps {
            beatmaps::create(beatmap);
        }
    }

    Ok(())
}

pub fn delete_cache(id: i64) -> Result<(), Error> {
    let beatmaps = beatmaps::get_mapset_maps(id);
    for beatmap in &beatmaps {
        beatmaps::delete(beatmap.id)?;
    }

    beatmapsets::delete(id)?;

    Ok(())
}

pub async fn get_beatmap(ctx: Context<'_>, id: u32) -> Result<Beatmap, Error> {
    let query_beatmap = beatmaps::get_single(i64::from(id));
    if let Ok(beatmap) = query_beatmap {
        if check_beatmap_valid_result(&beatmap) {
            return Ok(beatmap);
        }
        delete_cache(beatmap.beatmapset_id)?;
        cache_beatmapset(ctx, beatmap.beatmapset_id).await?;
        return Ok(beatmaps::get_single(i64::from(id)).unwrap());
    }
    cache_beatmapset_from_beatmap(ctx, i64::from(id)).await?;
    Ok(beatmaps::get_single(i64::from(id)).unwrap())
}

pub async fn get_beatmapset(ctx: Context<'_>, id: u32) -> Result<Beatmapset, Error> {
    let query_beatmapset = beatmapsets::read(i64::from(id));
    if let Ok(beatmapset) = query_beatmapset {
        if check_beatmapset_valid_result(&beatmapset) {
            return Ok(beatmapset);
        }
        delete_cache(beatmapset.id)?;
        cache_beatmapset(ctx, beatmapset.id).await?;
        return Ok(beatmapsets::read(i64::from(id)).unwrap());
    }
    cache_beatmapset(ctx, i64::from(id)).await?;
    Ok(beatmapsets::read(i64::from(id)).unwrap())
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
    let current_time = Utc::now().naive_utc();
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
