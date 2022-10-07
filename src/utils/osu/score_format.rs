use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::utils::osu::misc_format::format_beatmap_link;
use crate::utils::osu::pp::CalculateResults;
use num_format::{Locale, ToFormattedString};
use rosu_v2::model::GameMode;
use rosu_v2::prelude::Score;

pub fn format_score_statistic(
    score: &Score,
    beatmap: &Beatmap,
    pp: &Option<CalculateResults>,
) -> String {
    let sign = if score.accuracy.eq(&100.0) {
        "!"
    } else if score.perfect {
        "+"
    } else {
        "-"
    };

    let max_combo: i32;
    if let Some(pp) = pp {
        max_combo = match pp.max_combo {
            Some(calculated_combo) => calculated_combo as i32,
            _ => beatmap.max_combo,
        }
    } else {
        max_combo = beatmap.max_combo;
    }

    match score.mode {
        GameMode::Osu => {
            format!(
                "  acc    300s  100s  50s  miss  combo\
                \n{sign} {:<7}{:<6}{:<6}{:<5}{:<6}{}/{}",
                format!(
                    "{}%",
                    crate::utils::misc::remove_trailing_zeros(score.accuracy.into(), 2)
                ),
                score.statistics.count_300,
                score.statistics.count_100,
                score.statistics.count_50,
                score.statistics.count_miss,
                score.max_combo,
                max_combo
            )
        }
        GameMode::Taiko => format!(
            "  acc    great  good  miss  combo\
            \n{sign} {:<7}{:<7}{:<6}{:<6}{}/{}",
            format!(
                "{}%",
                crate::utils::misc::remove_trailing_zeros(score.accuracy.into(), 2)
            ),
            score.statistics.count_300,
            score.statistics.count_100,
            score.statistics.count_miss,
            score.max_combo,
            max_combo
        ),
        GameMode::Mania => format!(
            "  acc    max   300s  200s  100s  50s  miss\
        \n{sign} {:<7}{:<6}{:<6}{:<6}{:<6}{:<5}{:<6}",
            format!(
                "{}%",
                crate::utils::misc::remove_trailing_zeros(score.accuracy.into(), 2)
            ),
            score.statistics.count_geki,
            score.statistics.count_300,
            score.statistics.count_katu,
            score.statistics.count_100,
            score.statistics.count_50,
            score.statistics.count_miss
        ),
        GameMode::Catch => format!(
            "  acc    fruits ticks drpm miss combo\
           \n{sign} {:<7}{:<7}{:<6}{:<5}{:<5}{}/{}",
            format!(
                "{}%",
                crate::utils::misc::remove_trailing_zeros(score.accuracy.into(), 2)
            ),
            score.statistics.count_300,
            score.statistics.count_100,
            score.statistics.count_katu,
            score.statistics.count_miss,
            score.max_combo,
            max_combo
        ),
    }
}

pub fn format_score_info(
    score: &Score,
    beatmap: &Beatmap,
    beatmapset: &Beatmapset,
    pp: &Option<CalculateResults>,
) -> String {
    let italic = if beatmapset.artist.contains('*') {
        ""
    } else {
        "*"
    };

    let stars: f64;
    let score_pp: f64;
    if let Some(pp) = pp {
        stars = pp.total_stars;
        score_pp = match score.pp {
            Some(api_pp) => f64::from(api_pp),
            _ => pp.pp,
        }
    } else {
        stars = f64::from(beatmap.difficulty_rating);
        score_pp = f64::from(score.pp.unwrap_or(0.0));
    }

    format!(
        "[{italic}{} - {} [{}]{italic}]({})\n\
        **{}pp {}★, {} +{} {}**",
        beatmapset.artist,
        beatmapset.title,
        beatmap.version,
        format_beatmap_link(&beatmap.id, &beatmapset.id, &score.mode.to_string()),
        crate::utils::misc::remove_trailing_zeros(score_pp, 2),
        crate::utils::misc::remove_trailing_zeros(stars, 2),
        score.grade,
        score.mods,
        score.score.to_formatted_string(&Locale::en)
    )
}

pub fn format_new_score(
    score: &Score,
    beatmap: &Beatmap,
    beatmapset: &Beatmapset,
    pp: &Option<CalculateResults>,
) -> String {
    format!(
        "{}```diff\n{}```",
        format_score_info(score, beatmap, beatmapset, &pp),
        format_score_statistic(score, beatmap, &pp)
    )
}
