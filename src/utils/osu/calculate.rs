use crate::models::beatmaps::Beatmap;
use crate::utils::osu::misc::gamemode_from_string;
use crate::utils::osu::pp::catch::calculate_catch_pp;
use crate::utils::osu::pp::mania::calculate_mania_pp;
use crate::utils::osu::pp::osu::calculate_std_pp;
use crate::utils::osu::pp::taiko::calculate_taiko_pp;
use crate::utils::osu::pp::CalculateResults;
use crate::Error;
use rosu_v2::model::GameMode;
use std::io::Cursor;
use std::path::PathBuf;
use std::time::SystemTime;
use tokio::fs::create_dir_all;

const CACHE_PATH: &str = "osu_files/";

async fn download_beatmap(path: &PathBuf, map_id: i64) -> Result<(), Error> {
    let response = reqwest::get(format!("https://osu.ppy.sh/osu/{map_id}")).await?;
    let mut file = std::fs::File::create(path)?;
    let mut content = Cursor::new(response.bytes().await?);
    std::io::copy(&mut content, &mut file)?;

    Ok(())
}

async fn get_beatmap_bath(beatmap: &Beatmap) -> Result<PathBuf, Error> {
    let mut path = PathBuf::from(CACHE_PATH);
    let mut loved_path = PathBuf::from(CACHE_PATH.to_string() + "loved/");
    if !path.exists() | !loved_path.exists() {
        create_dir_all(&loved_path).await?;
    }
    match beatmap.status.as_str() {
        "Ranked" | "Approved" => {
            path.push(format!("{}.osu", beatmap.id));
            if !path.exists() {
                download_beatmap(&path, beatmap.id).await?;
            }
        }
        "Loved" => {
            loved_path.push(format!("{}.osu", beatmap.id));
            if !loved_path.exists() {
                download_beatmap(&loved_path, beatmap.id).await?;
            } else if (SystemTime::now()
                .duration_since(loved_path.metadata()?.modified()?)?
                .as_secs()
                / 60
                / 60
                / 24)
                > 30
            {
                download_beatmap(&loved_path, beatmap.id).await?;
            }
            return Ok(loved_path);
        }
        _ => {
            path.push("temp.osu");
            download_beatmap(&path, beatmap.id).await?;
        }
    };

    Ok(path)
}

pub async fn calculate(
    score: Option<&rosu_v2::prelude::Score>,
    beatmap: &Beatmap,
    potential_acc: Option<f64>,
) -> Result<CalculateResults, Error> {
    let path = get_beatmap_bath(beatmap).await?;
    if let Some(score) = score {
        return match score.mode {
            GameMode::Osu => Ok(calculate_std_pp(
                path,
                score.mods.bits(),
                Some(score.max_combo as usize),
                Some(f64::from(score.accuracy)),
                potential_acc,
                Some(score.statistics.count_300 as usize),
                Some(score.statistics.count_100 as usize),
                Some(score.statistics.count_50 as usize),
                Some(score.statistics.count_miss as usize),
                Some(score.total_hits() as usize),
                score.mods.clock_rate(),
            )
            .await),
            GameMode::Mania => Ok(calculate_mania_pp(
                path,
                score.mods.bits(),
                Some(score.statistics.count_geki as usize),
                Some(score.statistics.count_300 as usize),
                Some(score.statistics.count_katu as usize),
                Some(score.statistics.count_100 as usize),
                Some(score.statistics.count_50 as usize),
                Some(score.statistics.count_miss as usize),
                Some(score.total_hits() as usize),
                score.mods.clock_rate(),
            )
            .await),
            GameMode::Taiko => Ok(calculate_taiko_pp(
                path,
                score.mods.bits(),
                Some(score.max_combo as usize),
                Some(f64::from(score.accuracy)),
                Some(score.statistics.count_300 as usize),
                Some(score.statistics.count_100 as usize),
                Some(score.statistics.count_miss as usize),
                Some(score.total_hits() as usize),
                score.mods.clock_rate(),
            )
            .await),
            GameMode::Catch => Ok(calculate_catch_pp(
                path,
                score.mods.bits(),
                Some(score.max_combo as usize),
                Some(score.statistics.count_300 as usize),
                Some(score.statistics.count_100 as usize),
                Some(score.statistics.count_50 as usize),
                Some(score.statistics.count_katu as usize),
                Some(score.statistics.count_miss as usize),
                Some(score.total_hits() as usize),
                score.mods.clock_rate(),
            )
            .await),
        };
    }

    match gamemode_from_string(&beatmap.mode)
        .ok_or("Failed to parse beatmap mode in calculate_pp")?
    {
        GameMode::Osu => Ok(calculate_std_pp(
            path, 0, None, None, None, None, None, None, None, None, None,
        )
        .await),
        GameMode::Mania => {
            Ok(calculate_mania_pp(path, 0, None, None, None, None, None, None, None, None).await)
        }
        GameMode::Taiko => {
            Ok(calculate_taiko_pp(path, 0, None, None, None, None, None, None, None).await)
        }
        GameMode::Catch => {
            Ok(calculate_catch_pp(path, 0, None, None, None, None, None, None, None, None).await)
        }
    }
}
