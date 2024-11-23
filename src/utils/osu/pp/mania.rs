use crate::utils::osu::pp::{CalculateResults, ManiaScore};
use crate::Error;
use rosu_pp::mania::ManiaPerformance;
use rosu_pp::model::mode::GameMode;
use rosu_pp::Beatmap;

pub fn calculate_mania_pp(file: &[u8], score_state: ManiaScore) -> Result<CalculateResults, Error> {
    let binding = Beatmap::from_bytes(file)?;
    let map = binding.convert(GameMode::Mania, &score_state.mods)?;

    let (mut result, diff_attributes, full_difficulty) = if score_state.passed {
        let difficulty = ManiaPerformance::from(&map).mods(score_state.mods.clone());
        let diff_attributes = map.attributes().mods(score_state.mods);

        (difficulty, diff_attributes.build(), None)
    } else {
        let mut difficulty = ManiaPerformance::from(&map).mods(score_state.mods.clone());
        let diff_attributes = map.attributes().mods(score_state.mods);

        let full_difficulty = difficulty.clone().calculate()?;

        if let Some(passed_objects) = score_state.passed_objects {
            difficulty = difficulty.passed_objects(passed_objects);
        }

        (difficulty, diff_attributes.build(), Some(full_difficulty))
    };

    if let Some(nmiss) = score_state.nmiss {
        result = result.misses(nmiss);
    };

    if let Some(n320) = score_state.n320 {
        result = result.n320(n320);
    };

    if let Some(n300) = score_state.n300 {
        result = result.n300(n300);
    };

    if let Some(n200) = score_state.n200 {
        result = result.n100(n200);
    };

    if let Some(n100) = score_state.n100 {
        result = result.n100(n100);
    };

    if let Some(n50) = score_state.n50 {
        result = result.n50(n50);
    };

    let result = result.calculate()?;

    let full_calc = if let Some(full_difficulty) = full_difficulty {
        full_difficulty
    } else {
        result.clone()
    };

    Ok(CalculateResults {
        total_stars: full_calc.stars(),
        partial_stars: result.stars(),
        pp: result.pp,
        max_pp: None,
        max_combo: full_calc.max_combo(),
        clock_rate: diff_attributes.clock_rate,
    })
}
