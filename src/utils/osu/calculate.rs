use crate::models::beatmaps::Beatmap;
use crate::utils::osu::pp::catch::calculate_catch_pp;
use crate::utils::osu::pp::mania::calculate_mania_pp;
use crate::utils::osu::pp::osu::calculate_std_pp;
use crate::utils::osu::pp::taiko::calculate_taiko_pp;
use crate::utils::osu::pp::CalculateResults;
use crate::Error;
use rosu_v2::model::GameMode;
use std::fs::create_dir;
use std::io::Cursor;
use std::path::PathBuf;

const CACHE_PATH: &str = "osu_files/";

async fn download_beatmap(path: &PathBuf, map_id: i64) {
    let response = reqwest::get(format!("https://osu.ppy.sh/osu/{}", map_id))
        .await
        .expect("Couldn't download beatmap file.");
    let mut file = std::fs::File::create(path).expect("Couldn't create beatmap file");
    let mut content = Cursor::new(
        response
            .bytes()
            .await
            .expect("Couldn't download beatmap file."),
    );
    std::io::copy(&mut content, &mut file).expect("Couldn't save beatmap file.");
}

async fn get_beatmap_bath(beatmap: &Beatmap) -> Result<PathBuf, Error> {
    let mut path = PathBuf::from(CACHE_PATH);
    if !path.exists() {
        create_dir(&path).expect("Couldn't create path");
    }
    match beatmap.status.as_str() {
        "Ranked" | "Approved" => {
            path.push(format!("{}.osu", beatmap.id));
            if !path.exists() {
                download_beatmap(&path, beatmap.id).await;
            }
        }
        _ => {
            path.push("temp.osu");
            download_beatmap(&path, beatmap.id).await;
        }
    };

    Ok(path)
}

pub async fn calculate(
    score: &rosu_v2::prelude::Score,
    beatmap: &Beatmap,
    potential_acc: Option<f64>,
) -> Result<CalculateResults, Error> {
    match score.mode {
        GameMode::Osu => {
            let path = get_beatmap_bath(beatmap).await?;
            Ok(calculate_std_pp(
                path,
                score.mods.bits(),
                Some(score.max_combo as usize),
                Some(f64::from(score.accuracy)),
                potential_acc,
                Some(score.statistics.count_300 as usize),
                Some(score.statistics.count_100 as usize),
                Some(score.statistics.count_50 as usize),
                Some(score.statistics.count_miss as usize),
                None,
            )
            .await)
        }
        GameMode::Mania => {
            let path = get_beatmap_bath(beatmap).await?;
            Ok(calculate_mania_pp(path, score.mods.bits(), Some(score.score), None).await)
        }
        GameMode::Taiko => {
            let path = get_beatmap_bath(beatmap).await?;
            Ok(calculate_taiko_pp(
                path,
                score.mods.bits(),
                Some(score.max_combo as usize),
                Some(f64::from(score.accuracy)),
                Some(score.statistics.count_300 as usize),
                Some(score.statistics.count_100 as usize),
                Some(score.statistics.count_miss as usize),
                None,
            )
            .await)
        }
        GameMode::Catch => {
            let path = get_beatmap_bath(beatmap).await?;
            Ok(calculate_catch_pp(
                path,
                score.mods.bits(),
                Some(score.max_combo as usize),
                Some(score.statistics.count_300 as usize),
                Some(score.statistics.count_100 as usize),
                Some(score.statistics.count_50 as usize),
                Some(score.statistics.count_katu as usize),
                Some(score.statistics.count_miss as usize),
                None,
            )
            .await)
        }
    }
}
