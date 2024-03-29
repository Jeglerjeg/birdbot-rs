use crate::utils::osu::pp::{get_map_attributes, parse_map, CalculateResults};
use crate::Error;
use rosu_pp::{BeatmapExt, GameMode, OsuPP};

pub fn calculate_std_pp(
    file: &[u8],
    mods: u32,
    combo: Option<usize>,
    acc: Option<f64>,
    potential_acc: Option<f64>,
    n300: Option<usize>,
    n100: Option<usize>,
    n50: Option<usize>,
    nmiss: Option<usize>,
    passed_objects: Option<usize>,
    clock_rate: Option<f32>,
) -> Result<CalculateResults, Error> {
    let map = parse_map(file)?;

    let mut result = OsuPP::new(&map).mods(mods);

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
        result = result.n_misses(nmiss);
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

    let potential_result;
    if let (Some(n300), Some(n100), Some(n50), Some(nmiss)) = (n300, n100, n50, nmiss) {
        potential_result = OsuPP::new(&map)
            .mods(mods)
            .mode(GameMode::Osu)
            .n300(n300 + nmiss)
            .n100(n100)
            .n50(n50);
    } else if let Some(potential_acc) = potential_acc {
        potential_result = OsuPP::new(&map)
            .mods(mods)
            .mode(GameMode::Osu)
            .accuracy(potential_acc);
    } else {
        potential_result = OsuPP::new(&map).mods(mods).mode(GameMode::Osu);
    }

    let map_attributes = get_map_attributes(&map, GameMode::Catch, mods, clock_rate);

    let result = result.calculate();

    let mut map_calc = map.stars().mods(mods).mode(GameMode::Osu);

    if let Some(clock_rate) = clock_rate {
        map_calc = map_calc.clock_rate(f64::from(clock_rate));
    }

    let map_calc = map_calc.calculate();

    Ok(CalculateResults {
        total_stars: map_calc.stars(),
        partial_stars: result.stars(),
        pp: result.pp,
        max_pp: Some(potential_result.calculate().pp()),
        max_combo: map_calc.max_combo(),
        ar: map_attributes.ar,
        cs: map_attributes.cs,
        od: map_attributes.od,
        hp: map_attributes.hp,
        clock_rate: map_attributes.clock_rate,
    })
}
