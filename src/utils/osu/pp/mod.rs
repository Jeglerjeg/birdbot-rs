use rosu_pp::Beatmap;
use std::path::PathBuf;

pub mod catch;
pub mod mania;
pub mod osu;
pub mod taiko;

pub async fn parse_map(file_path: PathBuf) -> Beatmap {
    match Beatmap::from_path(file_path).await {
        Ok(map) => map,
        Err(why) => panic!("Error while parsing map: {why}"),
    }
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
