use rosu_v2::model::GameMode;
use rosu_v2::prelude::Score;

pub fn gamemode_from_string(mode: &str) -> Option<GameMode> {
    match mode.to_lowercase().as_str() {
        "osu" | "standard" | "std" | "osu!" => Some(GameMode::Osu),
        "taiko" | "osu!taiko" => Some(GameMode::Taiko),
        "mania" | "keys" | "osu!mania" => Some(GameMode::Mania),
        "catch" | "ctb" | "fruits" | "osu!catch" => Some(GameMode::Catch),
        _ => None,
    }
}

pub fn calculate_potential_acc(score: &Score) -> Option<f64> {
    match score.mode {
        GameMode::Osu => {
            let total_hits = score.statistics.total_hits(GameMode::Osu);
            let total_points = (score.statistics.count_50 * 50)
                + (score.statistics.count_100 * 100)
                + (score.statistics.count_300 + score.statistics.count_miss) * 300;
            Some((f64::from(total_points) / (f64::from(total_hits) * 300.0)) * 100.0)
        }
        _ => None,
    }
}

pub fn count_score_pages(scores: &[Score], scores_per_page: usize) -> usize {
    (scores.len() + scores_per_page - 1) / scores_per_page
}
