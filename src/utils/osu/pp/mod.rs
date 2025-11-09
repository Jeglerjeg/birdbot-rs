use rosu_pp::GameMods;

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
    pub od: Option<f64>,
    pub hp: Option<f64>,
    pub ar: Option<f64>,
    pub cs: Option<f64>,
}

pub struct StandardScore {
    pub mods: GameMods,
    pub passed: bool,
    pub combo: Option<u32>,
    pub acc: Option<f64>,
    pub potential_acc: Option<f64>,
    pub n300: Option<u32>,
    pub n100: Option<u32>,
    pub n50: Option<u32>,
    pub nmiss: Option<u32>,
    pub passed_objects: Option<u32>,
    pub n_slider_ticks: Option<u32>,
    pub n_small_tick_hit: Option<u32>,
    pub n_slider_ends: Option<u32>,
    pub lazer: bool,
}

impl Default for StandardScore {
    fn default() -> Self {
        StandardScore {
            mods: GameMods::default(),
            passed: true,
            combo: None,
            acc: None,
            potential_acc: None,
            n300: None,
            n100: None,
            n50: None,
            nmiss: None,
            passed_objects: None,
            n_slider_ends: None,
            n_small_tick_hit: None,
            n_slider_ticks: None,
            lazer: false,
        }
    }
}

pub struct ManiaScore {
    pub mods: GameMods,
    pub passed: bool,
    pub n320: Option<u32>,
    pub n300: Option<u32>,
    pub n200: Option<u32>,
    pub n100: Option<u32>,
    pub n50: Option<u32>,
    pub nmiss: Option<u32>,
    pub passed_objects: Option<u32>,
}

impl Default for ManiaScore {
    fn default() -> Self {
        ManiaScore {
            mods: GameMods::default(),
            passed: true,
            n320: None,
            n300: None,
            n200: None,
            n100: None,
            n50: None,
            nmiss: None,
            passed_objects: None,
        }
    }
}

pub struct CatchScore {
    pub mods: GameMods,
    pub passed: bool,
    pub combo: Option<u32>,
    pub fruits: Option<u32>,
    pub droplets: Option<u32>,
    pub tiny_droplets: Option<u32>,
    pub tiny_droplet_misses: Option<u32>,
    pub nmiss: Option<u32>,
    pub passed_objects: Option<u32>,
}

impl Default for CatchScore {
    fn default() -> Self {
        CatchScore {
            mods: GameMods::default(),
            passed: true,
            combo: None,
            fruits: None,
            droplets: None,
            tiny_droplets: None,
            tiny_droplet_misses: None,
            nmiss: None,
            passed_objects: None,
        }
    }
}

pub struct TaikoScore {
    pub mods: GameMods,
    pub passed: bool,
    pub combo: Option<u32>,
    pub acc: Option<f64>,
    pub n300: Option<u32>,
    pub n100: Option<u32>,
    pub nmiss: Option<u32>,
    pub passed_objects: Option<u32>,
}

impl Default for TaikoScore {
    fn default() -> Self {
        TaikoScore {
            mods: GameMods::default(),
            passed: true,
            combo: None,
            acc: None,
            n300: None,
            n100: None,
            nmiss: None,
            passed_objects: None,
        }
    }
}
