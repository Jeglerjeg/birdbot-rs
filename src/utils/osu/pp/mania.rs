use crate::utils::osu::pp::{get_map_attributes, parse_map, CalculateResults};
use crate::Error;
use rosu_pp::{BeatmapExt, GameMode, ManiaPP};

pub async fn calculate_mania_pp(
    file: &[u8],
    mods: u32,
    n320: Option<usize>,
    n300: Option<usize>,
    n200: Option<usize>,
    n100: Option<usize>,
    n50: Option<usize>,
    nmiss: Option<usize>,
    passed_objects: Option<usize>,
    clock_rate: Option<f32>,
) -> Result<CalculateResults, Error> {
    let map = parse_map(file).await?;
    let map = map.convert_mode(GameMode::Mania);

    let mut result = ManiaPP::new(&map).mods(mods);

    if let Some(passed_objects) = passed_objects {
        result = result.passed_objects(passed_objects);
    };

    if let Some(clock_rate) = clock_rate {
        result = result.clock_rate(f64::from(clock_rate));
    }

    if let Some(nmiss) = nmiss {
        result = result.n_misses(nmiss);
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

    let map_attributes = get_map_attributes(&map, GameMode::Catch, mods, clock_rate);

    let mut map_calc = map.stars().mods(mods).mode(GameMode::Mania);

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
