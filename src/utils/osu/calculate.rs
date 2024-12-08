use crate::models::beatmaps::Beatmap;
use crate::models::osu_files::OsuFile;
use crate::utils::osu::misc::gamemode_from_string;
use crate::utils::osu::pp::catch::calculate_catch_pp;
use crate::utils::osu::pp::mania::calculate_mania_pp;
use crate::utils::osu::pp::osu::calculate_std_pp;
use crate::utils::osu::pp::taiko::calculate_taiko_pp;
use crate::utils::osu::pp::{CalculateResults, CatchScore, ManiaScore, StandardScore, TaikoScore};
use crate::Error;
use rosu_v2::model::GameMode;

pub fn calculate(
    score: Option<&rosu_v2::prelude::Score>,
    beatmap: &Beatmap,
    osu_file: &OsuFile,
    potential_acc: Option<f64>,
) -> Result<CalculateResults, Error> {
    if let Some(score) = score {
        return match score.mode {
            GameMode::Osu => Ok(calculate_std_pp(
                &osu_file.file,
                StandardScore {
                    mods: score.mods.clone().into(),
                    passed: score.passed,
                    combo: Some(score.max_combo),
                    acc: Some(f64::from(score.accuracy)),
                    potential_acc,
                    n300: Some(score.statistics.great),
                    n100: Some(score.statistics.ok),
                    n50: Some(score.statistics.meh),
                    nmiss: Some(score.statistics.miss),
                    passed_objects: Some(score.total_hits()),
                    n_slider_ticks: Some(score.statistics.large_tick_hit),
                    n_small_tick_hit: Some(score.statistics.small_tick_hit),
                    n_slider_ends: Some(score.statistics.slider_tail_hit),
                    lazer: score.build_id.is_some(),
                },
            )?),
            GameMode::Mania => Ok(calculate_mania_pp(
                &osu_file.file,
                ManiaScore {
                    mods: score.mods.clone().into(),
                    passed: score.passed,
                    n320: Some(score.statistics.perfect),
                    n300: Some(score.statistics.great),
                    n200: Some(score.statistics.good),
                    n100: Some(score.statistics.ok),
                    n50: Some(score.statistics.meh),
                    nmiss: Some(score.statistics.miss),
                    passed_objects: Some(score.total_hits()),
                },
            )?),
            GameMode::Taiko => Ok(calculate_taiko_pp(
                &osu_file.file,
                TaikoScore {
                    mods: score.mods.clone().into(),
                    passed: score.passed,
                    combo: Some(score.max_combo),
                    acc: Some(f64::from(score.accuracy)),
                    n300: Some(score.statistics.great),
                    n100: Some(score.statistics.ok),
                    nmiss: Some(score.statistics.miss),
                    passed_objects: Some(score.total_hits()),
                },
            )?),
            GameMode::Catch => Ok(calculate_catch_pp(
                &osu_file.file,
                CatchScore {
                    mods: score.mods.clone().into(),
                    passed: score.passed,
                    combo: Some(score.max_combo),
                    fruits: Some(score.statistics.great),
                    droplets: Some(score.statistics.large_tick_hit),
                    tiny_droplets: Some(score.statistics.small_tick_hit),
                    tiny_droplet_misses: Some(score.statistics.small_tick_miss),
                    nmiss: Some(score.statistics.miss),
                    passed_objects: Some(score.total_hits()),
                },
            )?),
        };
    }

    match gamemode_from_string(&beatmap.mode)
        .ok_or("Failed to parse beatmap mode in calculate_pp")?
    {
        GameMode::Osu => Ok(calculate_std_pp(&osu_file.file, StandardScore::default())?),
        GameMode::Mania => Ok(calculate_mania_pp(&osu_file.file, ManiaScore::default())?),
        GameMode::Taiko => Ok(calculate_taiko_pp(&osu_file.file, TaikoScore::default())?),
        GameMode::Catch => Ok(calculate_catch_pp(&osu_file.file, CatchScore::default())?),
    }
}
