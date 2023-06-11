use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::utils::misc::remove_trailing_zeros;
use crate::utils::osu::misc::calculate_potential_acc;
use crate::utils::osu::misc_format::{format_beatmap_link, format_footer};
use crate::utils::osu::pp::CalculateResults;
use crate::Error;
use num_format::{Locale, ToFormattedString};
use rosu_v2::model::GameMode;
use rosu_v2::prelude::Score;

pub fn format_score_statistic(
    score: &Score,
    beatmap: &Beatmap,
    pp: &Option<CalculateResults>,
) -> Result<String, Error> {
    let color = if score.perfect {
        "\u{001b}[0;32m"
    } else {
        "\u{001b}[0;31m"
    };

    let max_combo: i64;
    if let Some(pp) = pp {
        max_combo = pp.max_combo as i64;
    } else {
        max_combo = i64::from(beatmap.max_combo);
    }

    match score.mode {
        GameMode::Osu => Ok(format!(
            "acc    300s  100s  50s  miss  combo\
                \n{color}{:<7}{:<6}{:<6}{:<5}{:<6}{}/{}",
            format!("{}%", remove_trailing_zeros(score.accuracy.into(), 2)?),
            score.statistics.count_300,
            score.statistics.count_100,
            score.statistics.count_50,
            score.statistics.count_miss,
            score.max_combo,
            max_combo
        )),
        GameMode::Taiko => Ok(format!(
            "acc    great  good  miss  combo\
            \n{color}{:<7}{:<7}{:<6}{:<6}{}/{}",
            format!("{}%", remove_trailing_zeros(score.accuracy.into(), 2)?),
            score.statistics.count_300,
            score.statistics.count_100,
            score.statistics.count_miss,
            score.max_combo,
            max_combo
        )),
        GameMode::Mania => Ok(format!(
            "acc    max   300s  200s  100s  50s  miss\
        \n{color}{:<7}{:<6}{:<6}{:<6}{:<6}{:<5}{:<6}",
            format!("{}%", remove_trailing_zeros(score.accuracy.into(), 2)?),
            score.statistics.count_geki,
            score.statistics.count_300,
            score.statistics.count_katu,
            score.statistics.count_100,
            score.statistics.count_50,
            score.statistics.count_miss
        )),
        GameMode::Catch => Ok(format!(
            "acc    fruits ticks drpm miss combo\
           \n{color}{:<7}{:<7}{:<6}{:<5}{:<5}{}/{}",
            format!("{}%", remove_trailing_zeros(score.accuracy.into(), 2)?),
            score.statistics.count_300,
            score.statistics.count_100,
            score.statistics.count_katu,
            score.statistics.count_miss,
            score.max_combo,
            max_combo
        )),
    }
}

pub fn format_score_info(
    score: &Score,
    beatmap: &Beatmap,
    beatmapset: &Beatmapset,
    pp: &Option<CalculateResults>,
    scoreboard_rank: Option<&usize>,
) -> Result<String, Error> {
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
        stars = beatmap.difficulty_rating;
        score_pp = f64::from(score.pp.unwrap_or(0.0));
    }

    let scoreboard_rank = match score.rank_global {
        Some(rank) => format!("#{rank} "),
        _ => match scoreboard_rank {
            Some(rank) => format!("#{rank} "),
            _ => String::new(),
        },
    };

    let grade = if &score.grade.to_string() != "F" && !score.passed {
        format!("{} (Failed)", score.grade)
    } else {
        score.grade.to_string()
    };

    Ok(format!(
        "[{italic}{} - {} [{}]{italic}]({})\n\
        **{}pp {}★, {} {}+{} {}**",
        beatmapset.artist,
        beatmapset.title,
        beatmap.version,
        format_beatmap_link(beatmap.id, beatmapset.id, &score.mode.to_string()),
        remove_trailing_zeros(score_pp, 2)?,
        remove_trailing_zeros(stars, 2)?,
        grade,
        scoreboard_rank,
        score.mods,
        score.score.to_formatted_string(&Locale::en)
    ))
}

pub fn format_new_score(
    score: &Score,
    beatmap: &Beatmap,
    beatmapset: &Beatmapset,
    pp: &Option<CalculateResults>,
    scoreboard_rank: Option<&usize>,
) -> Result<String, Error> {
    Ok(format!(
        "{}```ansi\n{}```",
        format_score_info(score, beatmap, beatmapset, pp, scoreboard_rank)?,
        format_score_statistic(score, beatmap, pp)?
    ))
}

pub async fn format_score_list(
    scores: &[(Score, usize, Beatmap, Beatmapset)],
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<String, Error> {
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(5);

    let mut formatted_list: Vec<String> = Vec::new();
    for (pos, score) in scores.iter().enumerate() {
        if pos < offset {
            continue;
        }
        if pos > (limit + offset) - 1 {
            break;
        }

        let beatmap = &score.2;

        let beatmapset = &score.3;

        let pp = crate::utils::osu::calculate::calculate(
            &score.0,
            beatmap,
            calculate_potential_acc(&score.0),
        )
        .await;

        let footer: String;
        let pp = if let Ok(pp) = pp {
            let formatted_footer = format_footer(&score.0, beatmap, &pp)?;
            if formatted_footer.is_empty() {
                footer = String::new();
            } else {
                footer = format!("\n{formatted_footer}");
            }
            Some(pp)
        } else {
            footer = String::new();
            None
        };

        let formatted_score = format_new_score(&score.0, beatmap, beatmapset, &pp, None)?;

        formatted_list.push(format!(
            "{}.\n{}<t:{}:R>{}\n",
            score.1,
            formatted_score,
            score.0.ended_at.unix_timestamp(),
            footer
        ));
    }

    Ok(formatted_list.join("\n"))
}
