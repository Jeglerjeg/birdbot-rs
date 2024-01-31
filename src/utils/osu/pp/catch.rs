use crate::utils::osu::pp::{get_map_attributes, parse_map, CalculateResults};
use crate::Error;
use rosu_pp::{BeatmapExt, CatchPP, GameMode};

pub async fn calculate_catch_pp(
    file: &[u8],
    mods: u32,
    combo: Option<usize>,
    fruits: Option<usize>,
    droplets: Option<usize>,
    tiny_droplets: Option<usize>,
    tiny_droplet_misses: Option<usize>,
    nmiss: Option<usize>,
    passed_objects: Option<usize>,
    clock_rate: Option<f32>,
) -> Result<CalculateResults, Error> {
    let map = parse_map(file).await?;
    let map = map.convert_mode(GameMode::Catch);

    let mut result = CatchPP::new(&map).mods(mods);

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

    let map_attributes = get_map_attributes(&map, GameMode::Catch, mods, clock_rate);

    let mut map_calc = map.stars().mods(mods).mode(GameMode::Catch);

    if let Some(clock_rate) = clock_rate {
        map_calc = map_calc.clock_rate(f64::from(clock_rate));
    }

    let map_calc = map_calc.calculate();

    Ok(CalculateResults {
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
    })
}
