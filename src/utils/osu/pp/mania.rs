use crate::utils::osu::pp::{parse_map, CalculateResults};
use rosu_pp::{BeatmapExt, GameMode, ManiaPP};
use std::path::PathBuf;

pub async fn calculate_mania_pp(
    file_path: PathBuf,
    mods: u32,
    score: Option<u32>,
    passed_objects: Option<usize>,
) -> CalculateResults {
    let map = parse_map(file_path).await;
    let map = map.convert_mode(GameMode::Mania);

    let mut result = ManiaPP::new(&map).mods(mods);

    if let Some(passed_objects) = passed_objects {
        result = result.passed_objects(passed_objects);
    };

    if let Some(score) = score {
        result = result.score(score);
    };

    let result = result.calculate();

    let map_attributes = map.attributes().mods(mods).mode(GameMode::Mania).build();

    let map_calc = map.stars().mods(mods).mode(GameMode::Mania).calculate();

    CalculateResults {
        total_stars: map_calc.stars(),
        partial_stars: result.stars(),
        pp: result.pp,
        max_pp: None,
        max_combo: map_calc.max_combo(),
        ar: map_attributes.ar,
        cs: map_attributes.cs,
        od: map_attributes.od,
        hp: map_attributes.hp,
        clock_rate: map_attributes.clock_rate,
    }
}
