use crate::utils::osu::pp::{parse_map, CalculateResults};
use rosu_pp::{BeatmapExt, GameMode, OsuPP};
use std::path::PathBuf;

pub async fn calculate_std_pp(
    file_path: PathBuf,
    mods: u32,
    combo: Option<usize>,
    acc: Option<f64>,
    potential_acc: Option<f64>,
    n300: Option<usize>,
    n100: Option<usize>,
    n50: Option<usize>,
    nmiss: Option<usize>,
    passed_objects: Option<usize>,
) -> CalculateResults {
    let map = parse_map(file_path).await;

    let mut result = OsuPP::new(&map).mods(mods);

    if let Some(passed_objects) = passed_objects {
        result = result.passed_objects(passed_objects);
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

    let potential_result = OsuPP::new(&map).mods(mods).mode(GameMode::Osu).n_misses(0);

    let map_attributes = map.attributes().mods(mods).build();

    let potential_result = match potential_acc {
        Some(x) => potential_result.accuracy(x),
        None => potential_result,
    };

    let result = result.calculate();

    let map_calc = map.stars().mods(mods).mode(GameMode::Osu).calculate();

    CalculateResults {
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
    }
}
