use crate::utils::osu::pp::CalculateResults;
use crate::Error;
use rosu_pp::catch::{Catch, CatchPerformance};
use rosu_pp::Beatmap;

pub fn calculate_catch_pp(
    file: &[u8],
    mods: u32,
    passed: bool,
    combo: Option<u32>,
    fruits: Option<u32>,
    droplets: Option<u32>,
    tiny_droplets: Option<u32>,
    tiny_droplet_misses: Option<u32>,
    nmiss: Option<u32>,
    passed_objects: Option<u32>,
    clock_rate: Option<f32>,
) -> Result<CalculateResults, Error> {
    let binding = Beatmap::from_bytes(file)?;
    let map = binding
        .try_as_converted::<Catch>()
        .ok_or("Couldn't convert map to catch")?;

    let (mut result, diff_attributes, full_difficulty) = if passed {
        let mut difficulty = CatchPerformance::from(&map).mods(mods);
        let mut diff_attributes = map.attributes().mods(mods);

        if let Some(clock_rate) = clock_rate {
            difficulty = difficulty.clock_rate(f64::from(clock_rate));
            diff_attributes = diff_attributes.clock_rate(f64::from(clock_rate));
        }

        (difficulty, diff_attributes.build(), None)
    } else {
        let mut difficulty = CatchPerformance::from(&map).mods(mods);
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
    }

    if let Some(clock_rate) = clock_rate {
        result = result.clock_rate(f64::from(clock_rate));
    }

    if let Some(combo) = combo {
        result = result.combo(combo);
    };

    if let Some(nmiss) = nmiss {
        result = result.misses(nmiss);
    };

    if let Some(fruits) = fruits {
        result = result.fruits(fruits);
    };

    if let Some(droplets) = droplets {
        result = result.droplets(droplets);
    };

    if let Some(tiny_droplets) = tiny_droplets {
        result = result.tiny_droplets(tiny_droplets);
    };

    if let Some(tiny_droplet_misses) = tiny_droplet_misses {
        result = result.tiny_droplet_misses(tiny_droplet_misses);
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
        ar: diff_attributes.ar,
        cs: diff_attributes.cs,
        od: diff_attributes.od,
        hp: diff_attributes.hp,
        clock_rate: diff_attributes.clock_rate,
    })
}
