use crate::models::beatmaps::Beatmap;
use crate::models::osu_files::OsuFile;
use crate::utils::osu::misc::gamemode_from_string;
use crate::utils::osu::pp::catch::calculate_catch_pp;
use crate::utils::osu::pp::mania::calculate_mania_pp;
use crate::utils::osu::pp::osu::calculate_std_pp;
use crate::utils::osu::pp::taiko::calculate_taiko_pp;
use crate::utils::osu::pp::CalculateResults;
use crate::Error;
use rosu_pp::GameMods;
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
                score.mods.clone().into(),
                score.passed,
                Some(score.max_combo),
                Some(f64::from(score.accuracy)),
                potential_acc,
                Some(score.statistics.great),
                Some(score.statistics.ok),
                Some(score.statistics.meh),
                Some(score.statistics.miss),
                Some(score.total_hits()),
            )?),
            GameMode::Mania => Ok(calculate_mania_pp(
                &osu_file.file,
                score.mods.clone().into(),
                score.passed,
                Some(score.statistics.perfect),
                Some(score.statistics.great),
                Some(score.statistics.good),
                Some(score.statistics.ok),
                Some(score.statistics.meh),
                Some(score.statistics.miss),
                Some(score.total_hits()),
            )?),
            GameMode::Taiko => Ok(calculate_taiko_pp(
                &osu_file.file,
                score.mods.clone().into(),
                score.passed,
                Some(score.max_combo),
                Some(f64::from(score.accuracy)),
                Some(score.statistics.great),
                Some(score.statistics.ok),
                Some(score.statistics.miss),
                Some(score.total_hits()),
            )?),
            GameMode::Catch => Ok(calculate_catch_pp(
                &osu_file.file,
                score.mods.clone().into(),
                score.passed,
                Some(score.max_combo),
                Some(score.statistics.great),
                Some(score.statistics.large_tick_hit),
                Some(score.statistics.small_tick_hit),
                Some(score.statistics.small_tick_miss),
                Some(score.statistics.miss),
                Some(score.total_hits()),
            )?),
        };
    }

    match gamemode_from_string(&beatmap.mode)
        .ok_or("Failed to parse beatmap mode in calculate_pp")?
    {
        GameMode::Osu => Ok(calculate_std_pp(
            &osu_file.file,
            GameMods::default(),
            true,
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
            GameMods::default(),
            true,
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
            GameMods::default(),
            true,
            None,
            None,
            None,
            None,
            None,
            None,
        )?),
        GameMode::Catch => Ok(calculate_catch_pp(
            &osu_file.file,
            GameMods::default(),
            true,
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
