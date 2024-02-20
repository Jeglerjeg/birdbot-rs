use crate::Error;
use rosu_pp::beatmap::BeatmapAttributes;
use rosu_pp::{Beatmap, GameMode};

pub mod catch;
pub mod mania;
pub mod osu;
pub mod taiko;

pub fn parse_map(file: &[u8]) -> Result<Beatmap, Error> {
    let beatmap = Beatmap::from_bytes(file)?;

    Ok(beatmap)
}

pub struct CalculateResults {
    pub total_stars: f64,
    pub partial_stars: f64,
    pub pp: f64,
    pub max_pp: Option<f64>,
    pub max_combo: usize,
    pub ar: f64,
    pub cs: f64,
    pub od: f64,
    pub hp: f64,
    pub clock_rate: f64,
}

fn get_map_attributes(
    beatmap: &Beatmap,
    mode: GameMode,
    mods: u32,
    clock_rate: Option<f32>,
) -> BeatmapAttributes {
    let mut map_attributes = beatmap.attributes();

    let map_attributes = if let Some(clock_rate) = clock_rate {
        map_attributes
            .mods(mods)
            .mode(mode)
            .clock_rate(f64::from(clock_rate))
    } else {
        map_attributes.mods(mods).mode(GameMode::Catch)
    };

    map_attributes.build()
}
