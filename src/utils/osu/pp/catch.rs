use crate::utils::osu::pp::{parse_map, CalculateResults};
use rosu_pp::{BeatmapExt, CatchPP, GameMode};
use std::path::PathBuf;

pub async fn calculate_catch_pp(
    file_path: PathBuf,
    mods: u32,
    combo: Option<usize>,
    fruits: Option<usize>,
    droplets: Option<usize>,
    tiny_droplets: Option<usize>,
    tiny_droplet_misses: Option<usize>,
    nmiss: Option<usize>,
    passed_objects: Option<usize>,
) -> CalculateResults {
    let map = parse_map(file_path).await;
    let map = map.convert_mode(GameMode::Catch);

    let mut result = CatchPP::new(&map).mods(mods);

    if let Some(passed_objects) = passed_objects {
        result = result.passed_objects(passed_objects);
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

    let map_attributes = map.attributes().mods(mods).mode(GameMode::Catch).build();

    let map_calc = map.stars().mods(mods).mode(GameMode::Catch).calculate();

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
