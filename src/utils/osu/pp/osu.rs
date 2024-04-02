use crate::utils::osu::pp::CalculateResults;
use crate::Error;
use rosu_pp::osu::{Osu, OsuPerformance, OsuPerformanceAttributes};
use rosu_pp::Beatmap;

pub fn calculate_std_pp(
    file: &[u8],
    mods: u32,
    passed: bool,
    combo: Option<u32>,
    acc: Option<f64>,
    potential_acc: Option<f64>,
    n300: Option<u32>,
    n100: Option<u32>,
    n50: Option<u32>,
    nmiss: Option<u32>,
    passed_objects: Option<u32>,
    clock_rate: Option<f32>,
) -> Result<CalculateResults, Error> {
    let binding = Beatmap::from_bytes(file)?;
    let map = binding
        .try_as_converted::<Osu>()
        .ok_or("Couldn't convert map to standard")?;

    let (mut result, diff_attributes, full_difficulty) = if passed {
        let mut difficulty = OsuPerformance::from(&map).mods(mods);
        let mut diff_attributes = map.attributes().mods(mods);

        if let Some(clock_rate) = clock_rate {
            difficulty = difficulty.clock_rate(f64::from(clock_rate));
            diff_attributes = diff_attributes.clock_rate(f64::from(clock_rate));
        }

        (difficulty, diff_attributes.build(), None)
    } else {
        let mut difficulty = OsuPerformance::from(&map).mods(mods);
        let mut diff_attributes = map.attributes().mods(mods);

        if let Some(clock_rate) = clock_rate {
            difficulty = difficulty.clock_rate(f64::from(clock_rate));
            diff_attributes = diff_attributes.clock_rate(f64::from(clock_rate));
        }

        let full_difficulty = difficulty.clone().calculate();

        if let Some(passed_objects) = passed_objects {
            difficulty = difficulty.passed_objects(passed_objects);
        }

        (difficulty, diff_attributes.build(), Some(full_difficulty))
    };

    if let Some(combo) = combo {
        result = result.combo(combo);
    };

    if let Some(nmiss) = nmiss {
        result = result.misses(nmiss);
    };

    if let Some(n300) = n300 {
        result = result.n300(n300);
    };

    if let Some(n100) = n100 {
        result = result.n100(n100);
    };

    if let Some(n50) = n50 {
        result = result.n50(n50);
    };

    if let Some(acc) = acc {
        result = result.accuracy(acc);
    };

    let result = result.calculate();

    let (full_calc, potential_result) = if let Some(full_difficulty) = full_difficulty {
        (
            full_difficulty.clone(),
            get_potential_pp(mods, potential_acc, n300, n100, n50, nmiss, full_difficulty),
        )
    } else {
        (
            result.clone(),
            get_potential_pp(mods, potential_acc, n300, n100, n50, nmiss, result.clone()),
        )
    };

    Ok(CalculateResults {
        total_stars: full_calc.stars(),
        partial_stars: result.stars(),
        pp: result.pp,
        max_pp: Some(potential_result),
        max_combo: full_calc.max_combo(),
        ar: diff_attributes.ar,
        cs: diff_attributes.cs,
        od: diff_attributes.od,
        hp: diff_attributes.hp,
        clock_rate: diff_attributes.clock_rate,
    })
}

fn get_potential_pp(
    mods: u32,
    potential_acc: Option<f64>,
    n300: Option<u32>,
    n100: Option<u32>,
    n50: Option<u32>,
    nmiss: Option<u32>,
    difficulty_attribs: OsuPerformanceAttributes,
) -> f64 {
    let potential_result;
    if let (Some(n300), Some(n100), Some(n50), Some(nmiss)) = (n300, n100, n50, nmiss) {
        potential_result = OsuPerformance::new(difficulty_attribs)
            .mods(mods)
            .n300(n300 + nmiss)
            .n100(n100)
            .n50(n50);
    } else if let Some(potential_acc) = potential_acc {
        potential_result = OsuPerformance::new(difficulty_attribs)
            .mods(mods)
            .accuracy(potential_acc);
    } else {
        potential_result = OsuPerformance::new(difficulty_attribs).mods(mods);
    }
    potential_result.calculate().pp()
}
