use crate::models::beatmaps::Beatmap;
use crate::models::osu_files::OsuFile;
use crate::utils::osu::misc::gamemode_from_string;
use crate::utils::osu::pp::catch::calculate_catch_pp;
use crate::utils::osu::pp::mania::calculate_mania_pp;
use crate::utils::osu::pp::osu::calculate_std_pp;
use crate::utils::osu::pp::taiko::calculate_taiko_pp;
use crate::utils::osu::pp::CalculateResults;
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
                score.mods.bits(),
                Some(score.max_combo as usize),
                Some(f64::from(score.accuracy)),
                potential_acc,
                Some(score.statistics.great as usize),
                Some(score.statistics.ok as usize),
                Some(score.statistics.meh as usize),
                Some(score.statistics.miss as usize),
                Some(score.total_hits() as usize),
                score.mods.clock_rate(),
            )?),
            GameMode::Mania => Ok(calculate_mania_pp(
                &osu_file.file,
                score.mods.bits(),
                Some(score.statistics.perfect as usize),
                Some(score.statistics.great as usize),
                Some(score.statistics.good as usize),
                Some(score.statistics.ok as usize),
                Some(score.statistics.meh as usize),
                Some(score.statistics.miss as usize),
                Some(score.total_hits() as usize),
                score.mods.clock_rate(),
            )?),
            GameMode::Taiko => Ok(calculate_taiko_pp(
                &osu_file.file,
                score.mods.bits(),
                Some(score.max_combo as usize),
                Some(f64::from(score.accuracy)),
                Some(score.statistics.great as usize),
                Some(score.statistics.ok as usize),
                Some(score.statistics.miss as usize),
                Some(score.total_hits() as usize),
                score.mods.clock_rate(),
            )?),
            GameMode::Catch => Ok(calculate_catch_pp(
                &osu_file.file,
                score.mods.bits(),
                Some(score.max_combo as usize),
                Some(score.statistics.great as usize),
                Some(score.statistics.large_tick_hit as usize),
                Some(score.statistics.small_tick_hit as usize),
                Some(score.statistics.small_tick_miss as usize),
                Some(score.statistics.miss as usize),
                Some(score.total_hits() as usize),
                score.mods.clock_rate(),
            )?),
        };
    }

    match gamemode_from_string(&beatmap.mode)
        .ok_or("Failed to parse beatmap mode in calculate_pp")?
    {
        GameMode::Osu => Ok(calculate_std_pp(
            &osu_file.file,
            0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )?),
        GameMode::Mania => Ok(calculate_mania_pp(
            &osu_file.file,
            0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )?),
        GameMode::Taiko => Ok(calculate_taiko_pp(
            &osu_file.file,
            0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )?),
        GameMode::Catch => Ok(calculate_catch_pp(
            &osu_file.file,
            0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )?),
    }
}
