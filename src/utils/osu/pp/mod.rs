pub mod catch;
pub mod mania;
pub mod osu;
pub mod taiko;

pub struct CalculateResults {
    pub total_stars: f64,
    pub partial_stars: f64,
    pub pp: f64,
    pub max_pp: Option<f64>,
    pub max_combo: u32,
    pub clock_rate: f64,
}
