use crate::models::beatmaps::Beatmap;
use crate::utils::osu::misc::gamemode_from_string;
use crate::utils::osu::pp::catch::calculate_catch_pp;
use crate::utils::osu::pp::mania::calculate_mania_pp;
use crate::utils::osu::pp::osu::calculate_std_pp;
use crate::utils::osu::pp::taiko::calculate_taiko_pp;
use crate::utils::osu::pp::CalculateResults;
use crate::Error;
use rosu_v2::model::GameMode;

pub async fn calculate(
    score: Option<&rosu_v2::prelude::Score>,
    beatmap: &Beatmap,
    potential_acc: Option<f64>,
) -> Result<CalculateResults, Error> {
    if let Some(score) = score {
        return match score.mode {
            GameMode::Osu => Ok(calculate_std_pp(
                &beatmap.osu_file,
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
            .await?),
            GameMode::Mania => Ok(calculate_mania_pp(
                &beatmap.osu_file,
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
            .await?),
            GameMode::Taiko => Ok(calculate_taiko_pp(
                &beatmap.osu_file,
                score.mods.bits(),
                Some(score.max_combo as usize),
                Some(f64::from(score.accuracy)),
                Some(score.statistics.count_300 as usize),
                Some(score.statistics.count_100 as usize),
                Some(score.statistics.count_miss as usize),
                Some(score.total_hits() as usize),
                score.mods.clock_rate(),
            )
            .await?),
            GameMode::Catch => Ok(calculate_catch_pp(
                &beatmap.osu_file,
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
            .await?),
        };
    }

    match gamemode_from_string(&beatmap.mode)
        .ok_or("Failed to parse beatmap mode in calculate_pp")?
    {
        GameMode::Osu => Ok(calculate_std_pp(
            &beatmap.osu_file,
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
        )
        .await?),
        GameMode::Mania => Ok(calculate_mania_pp(
            &beatmap.osu_file,
            0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await?),
        GameMode::Taiko => Ok(calculate_taiko_pp(
            &beatmap.osu_file,
            0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await?),
        GameMode::Catch => Ok(calculate_catch_pp(
            &beatmap.osu_file,
            0,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await?),
    }
}
