use crate::utils::osu::pp::CalculateResults;
use crate::Error;
use rosu_pp::mania::{Mania, ManiaPerformance};
use rosu_pp::Beatmap;

pub fn calculate_mania_pp(
    file: &[u8],
    mods: u32,
    passed: bool,
    n320: Option<u32>,
    n300: Option<u32>,
    n200: Option<u32>,
    n100: Option<u32>,
    n50: Option<u32>,
    nmiss: Option<u32>,
    passed_objects: Option<u32>,
    clock_rate: Option<f32>,
) -> Result<CalculateResults, Error> {
    let binding = Beatmap::from_bytes(file)?;
    let map = binding
        .try_as_converted::<Mania>()
        .ok_or("Couldn't convert map to mania")?;

    let (mut result, diff_attributes, full_difficulty) = if passed {
        let mut difficulty = ManiaPerformance::from(&map).mods(mods);
        let mut diff_attributes = map.attributes().mods(mods);

        if let Some(clock_rate) = clock_rate {
            difficulty = difficulty.clock_rate(f64::from(clock_rate));
            diff_attributes = diff_attributes.clock_rate(f64::from(clock_rate));
        }

        (difficulty, diff_attributes.build(), None)
    } else {
        let mut difficulty = ManiaPerformance::from(&map).mods(mods);
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

    if let Some(passed_objects) = passed_objects {
        result = result.passed_objects(passed_objects);
    };

    if let Some(clock_rate) = clock_rate {
        result = result.clock_rate(f64::from(clock_rate));
    }

    if let Some(nmiss) = nmiss {
        result = result.misses(nmiss);
    };

    if let Some(n320) = n320 {
        result = result.n320(n320);
    };

    if let Some(n300) = n300 {
        result = result.n300(n300);
    };

    if let Some(n200) = n200 {
        result = result.n100(n200);
    };

    if let Some(n100) = n100 {
        result = result.n100(n100);
    };

    if let Some(n50) = n50 {
        result = result.n50(n50);
    };

    let result = result.calculate();

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
