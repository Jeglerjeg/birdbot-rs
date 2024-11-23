use crate::utils::osu::pp::{CalculateResults, CatchScore};
use crate::Error;
use rosu_pp::catch::CatchPerformance;
use rosu_pp::model::mode::GameMode;
use rosu_pp::Beatmap;

pub fn calculate_catch_pp(file: &[u8], score_state: CatchScore) -> Result<CalculateResults, Error> {
    let binding = Beatmap::from_bytes(file)?;
    let map = binding.convert(GameMode::Catch, &score_state.mods)?;

    let (mut result, diff_attributes, full_difficulty) = if score_state.passed {
        let difficulty = CatchPerformance::from(&map).mods(score_state.mods.clone());
        let diff_attributes = map.attributes().mods(score_state.mods);

        (difficulty, diff_attributes.build(), None)
    } else {
        let mut difficulty = CatchPerformance::from(&map).mods(score_state.mods.clone());
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

    if let Some(fruits) = score_state.fruits {
        result = result.fruits(fruits);
    };

    if let Some(droplets) = score_state.droplets {
        result = result.droplets(droplets);
    };

    if let Some(tiny_droplets) = score_state.tiny_droplets {
        result = result.tiny_droplets(tiny_droplets);
    };

    if let Some(tiny_droplet_misses) = score_state.tiny_droplet_misses {
        result = result.tiny_droplet_misses(tiny_droplet_misses);
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
