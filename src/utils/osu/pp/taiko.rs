use crate::utils::osu::pp::{CalculateResults, TaikoScore};
use crate::Error;
use rosu_pp::model::mode::GameMode;
use rosu_pp::taiko::TaikoPerformance;
use rosu_pp::Beatmap;

pub fn calculate_taiko_pp(file: &[u8], score_state: TaikoScore) -> Result<CalculateResults, Error> {
    let binding = Beatmap::from_bytes(file)?;
    let map = binding.convert(GameMode::Taiko, &score_state.mods)?;

    let (mut result, diff_attributes, full_difficulty) = if score_state.passed {
        let difficulty = TaikoPerformance::from(&map).mods(score_state.mods.clone());
        let diff_attributes = map.attributes().mods(score_state.mods);

        (difficulty, diff_attributes.build(), None)
    } else {
        let mut difficulty = TaikoPerformance::from(&map).mods(score_state.mods.clone());
        let diff_attributes = map.attributes().mods(score_state.mods);

        let full_difficulty = difficulty.clone().calculate()?;

        if let Some(passed_objects) = score_state.passed_objects {
            difficulty = difficulty.passed_objects(passed_objects);
        }

        (difficulty, diff_attributes.build(), Some(full_difficulty))
    };

    if let Some(combo) = score_state.combo {
        result = result.combo(combo);
    };

    if let Some(nmiss) = score_state.nmiss {
        result = result.misses(nmiss);
    };

    if let Some(n300) = score_state.n300 {
        result = result.n300(n300);
    };

    if let Some(n100) = score_state.n100 {
        result = result.n100(n100);
    };

    if let Some(acc) = score_state.acc {
        result = result.accuracy(acc);
    };

    let result = result.calculate()?;

    let full_calc = if let Some(full_difficulty) = full_difficulty {
        full_difficulty
    } else {
        result.clone()
    };

    Ok(CalculateResults {
        total_stars: full_calc.stars(),
        partial_stars: result.difficulty.stars,
        pp: result.pp,
        max_pp: None,
        max_combo: full_calc.max_combo(),
        clock_rate: diff_attributes.clock_rate,
    })
}
