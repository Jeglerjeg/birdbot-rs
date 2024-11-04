use crate::utils::osu::pp::{CalculateResults, StandardScore};
use crate::Error;
use rosu_pp::osu::{Osu, OsuPerformance, OsuPerformanceAttributes};
use rosu_pp::Beatmap;

pub fn calculate_std_pp(
    file: &[u8],
    score_state: StandardScore,
) -> Result<CalculateResults, Error> {
    let binding = Beatmap::from_bytes(file)?;
    let map = binding
        .try_as_converted::<Osu>()
        .ok_or("Couldn't convert map to standard")?;

    let (mut result, diff_attributes, full_difficulty) = if score_state.passed {
        let difficulty = OsuPerformance::from(&map).mods(score_state.mods.clone());
        let diff_attributes = map.attributes().mods(score_state.mods.clone());

        (difficulty, diff_attributes.build(), None)
    } else {
        let mut difficulty = OsuPerformance::from(&map).mods(score_state.mods.clone());
        let diff_attributes = map.attributes().mods(score_state.mods.clone());

        let full_difficulty = difficulty.clone().calculate();

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

    if let Some(n50) = score_state.n50 {
        result = result.n50(n50);
    };

    if score_state.lazer {
        if let Some(n_slider_ends) = score_state.n_slider_ends {
            result = result.n_slider_ends(n_slider_ends);
        };

        if let Some(n_slider_ticks) = score_state.n_slider_ticks {
            result = result.n_slider_ticks(n_slider_ticks);
        };
    }

    if let Some(acc) = score_state.acc {
        result = result.accuracy(acc);
    };

    let result = result.calculate();

    let (full_calc, potential_result) = if let Some(full_difficulty) = full_difficulty {
        (
            full_difficulty.clone(),
            get_potential_pp(score_state, full_difficulty),
        )
    } else {
        (
            result.clone(),
            get_potential_pp(score_state, result.clone()),
        )
    };

    Ok(CalculateResults {
        total_stars: full_calc.stars(),
        partial_stars: result.stars(),
        pp: result.pp,
        max_pp: Some(potential_result),
        max_combo: full_calc.max_combo(),
        clock_rate: diff_attributes.clock_rate,
    })
}

fn get_potential_pp(
    score_state: StandardScore,
    difficulty_attribs: OsuPerformanceAttributes,
) -> f64 {
    let potential_result;
    if let (
        Some(n300),
        Some(n100),
        Some(n50),
        Some(nmiss),
        Some(n_slider_ends),
        Some(n_slider_ticks),
    ) = (
        score_state.n300,
        score_state.n100,
        score_state.n50,
        score_state.nmiss,
        score_state.n_slider_ends,
        score_state.n_slider_ticks,
    ) {
        if score_state.lazer {
            potential_result = OsuPerformance::new(difficulty_attribs)
                .mods(score_state.mods)
                .n300(n300 + nmiss)
                .n100(n100)
                .n50(n50)
                .n_slider_ends(n_slider_ends)
                .n_slider_ticks(n_slider_ticks);
        } else {
            potential_result = OsuPerformance::new(difficulty_attribs)
                .mods(score_state.mods)
                .n300(n300 + nmiss)
                .n100(n100)
                .n50(n50);
        }
    } else if let Some(potential_acc) = score_state.potential_acc {
        potential_result = OsuPerformance::new(difficulty_attribs)
            .mods(score_state.mods)
            .accuracy(potential_acc);
    } else {
        potential_result = OsuPerformance::new(difficulty_attribs).mods(score_state.mods);
    }
    potential_result.calculate().pp()
}
