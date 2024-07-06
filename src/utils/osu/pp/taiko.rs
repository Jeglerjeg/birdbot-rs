use crate::utils::osu::pp::CalculateResults;
use crate::Error;
use rosu_pp::taiko::Taiko;
use rosu_pp::taiko::TaikoPerformance;
use rosu_pp::{Beatmap, GameMods};

pub fn calculate_taiko_pp(
    file: &[u8],
    mods: GameMods,
    passed: bool,
    combo: Option<u32>,
    acc: Option<f64>,
    n300: Option<u32>,
    n100: Option<u32>,
    nmiss: Option<u32>,
    passed_objects: Option<u32>,
) -> Result<CalculateResults, Error> {
    let binding = Beatmap::from_bytes(file)?;
    let map = binding
        .try_as_converted::<Taiko>()
        .ok_or("Couldn't convert map to taiko")?;

    let (mut result, diff_attributes, full_difficulty) = if passed {
        let difficulty = TaikoPerformance::from(&map).mods(mods.clone());
        let diff_attributes = map.attributes().mods(mods);

        (difficulty, diff_attributes.build(), None)
    } else {
        let mut difficulty = TaikoPerformance::from(&map).mods(mods.clone());
        let diff_attributes = map.attributes().mods(mods);

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

    if let Some(acc) = acc {
        result = result.accuracy(acc);
    };

    let result = result.calculate();

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
