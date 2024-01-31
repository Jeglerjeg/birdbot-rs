use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::utils::misc::remove_trailing_zeros;
use crate::utils::osu::misc::is_perfect;
use crate::utils::osu::misc_format::{format_beatmap_link, format_footer};
use crate::utils::osu::pp::CalculateResults;
use crate::Error;
use num_format::{Locale, ToFormattedString};
use rosu_v2::model::GameMode;
use rosu_v2::prelude::Score;
use std::cmp;

pub fn format_score_statistic(score: &Score, pp: &CalculateResults) -> Result<String, Error> {
    let color = match score.build_id {
        None => {
            if score.legacy_perfect.ok_or(format!(
                "Couldn't get legacy_perfect for score id: {}",
                score.id
            ))? {
                "\u{001b}[0;32m"
            } else {
                "\u{001b}[0;31m"
            }
        }
        Some(_) => {
            if is_perfect(&score.statistics) {
                "\u{001b}[0;32m"
            } else {
                "\u{001b}[0;31m"
            }
        }
    };

    let max_combo = pp.max_combo;
    let accuracy_string = format!("{}%", remove_trailing_zeros(score.accuracy.into(), 2)?);
    let gap = if accuracy_string.len() < 6 {
        " ".repeat(cmp::max(accuracy_string.len() - 3 + 1, 2))
    } else {
        " ".repeat(4)
    };

    let stat_gap = if accuracy_string.len() > 3 {
        " ".repeat(gap.len() - (accuracy_string.len() - 3))
    } else {
        " ".repeat(gap.len())
    };

    match score.mode {
        GameMode::Osu => Ok(format!(
            "acc{gap}300s  100s  50s  miss  combo\
                \n{color}{}{stat_gap}{:<6}{:<6}{:<5}{:<6}{}/{}",
            accuracy_string,
            score.statistics.great,
            score.statistics.ok,
            score.statistics.meh,
            score.statistics.miss,
            score.max_combo,
            max_combo
        )),
        GameMode::Taiko => Ok(format!(
            "acc{gap}great  good  miss  combo\
            \n{color}{}{stat_gap}{:<7}{:<6}{:<6}{}/{}",
            accuracy_string,
            score.statistics.great,
            score.statistics.ok,
            score.statistics.miss,
            score.max_combo,
            max_combo
        )),
        GameMode::Mania => Ok(format!(
            "acc{gap}max   300s  200s  100s  50s  miss\
        \n{color}{}{stat_gap}{:<6}{:<6}{:<6}{:<6}{:<5}{:<6}",
            accuracy_string,
            score.statistics.perfect,
            score.statistics.great,
            score.statistics.good,
            score.statistics.ok,
            score.statistics.meh,
            score.statistics.miss
        )),
        GameMode::Catch => Ok(format!(
            "acc{gap}fruits ticks drpm miss combo\
           \n{color}{}{stat_gap}{:<7}{:<6}{:<5}{:<5}{}/{}",
            accuracy_string,
            score.statistics.great,
            score.statistics.large_tick_hit,
            score.statistics.small_tick_hit,
            score.statistics.small_tick_miss,
            score.max_combo,
            max_combo
        )),
    }
}

pub fn format_score_info(
    score: &Score,
    beatmap: &Beatmap,
    beatmapset: &Beatmapset,
    pp: &CalculateResults,
    scoreboard_rank: Option<&usize>,
) -> Result<String, Error> {
    let italic = if beatmapset.artist.contains('*') {
        ""
    } else {
        "*"
    };

    let stars = pp.total_stars;
    let score_pp = match score.pp {
        Some(api_pp) => f64::from(api_pp),
        _ => pp.pp,
    };

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
        format_beatmap_link(
            Some(beatmap.id),
            beatmapset.id,
            Some(&score.mode.to_string())
        ),
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
    pp: &CalculateResults,
    scoreboard_rank: Option<&usize>,
) -> Result<String, Error> {
    Ok(format!(
        "{}```ansi\n{}```",
        format_score_info(score, beatmap, beatmapset, pp, scoreboard_rank)?,
        format_score_statistic(score, pp)?
    ))
}

pub fn format_score_list(
    scores: &[(Score, usize, Beatmap, Beatmapset, CalculateResults)],
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<String, Error> {
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(5);

    let mut formatted_list: Vec<String> = Vec::new();
    for (pos, (score, position, beatmap, beatmapset, pp)) in scores.iter().enumerate() {
        if pos < offset {
            continue;
        }
        if pos > (limit + offset) - 1 {
            break;
        }

        let formatted_footer = format_footer(score, beatmap, pp)?;
        let footer = if formatted_footer.is_empty() {
            String::new()
        } else {
            format!("\n{formatted_footer}")
        };

        let formatted_score = format_new_score(score, beatmap, beatmapset, pp, None)?;

        formatted_list.push(format!(
            "{}.\n{}<t:{}:R>{}\n",
            position,
            formatted_score,
            score.ended_at.unix_timestamp(),
            footer
        ));
    }

    Ok(formatted_list.join("\n"))
}
