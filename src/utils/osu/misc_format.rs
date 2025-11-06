use crate::models::beatmaps::Beatmap;
use crate::models::osu_users::OsuUser;
use crate::utils::misc::remove_trailing_zeros;
use crate::utils::osu::misc::{DiffTypes, get_stat_diff};
use crate::utils::osu::pp::CalculateResults;
use crate::{Context, Error};
use aformat::{ArrayString, CapStr, aformat};
use num_format::{Locale, ToFormattedString};
use poise::serenity_prelude::User;
use rosu_v2::model::beatmap::RankStatus;
use rosu_v2::model::{GameMode, Grade};
use rosu_v2::prelude::{GameMod, GameMods, Score};

#[inline]
fn format_speed_change(speed_change: f64, acronym: &str) -> Result<String, Error> {
    Ok(aformat!(
        "{} ({}x)",
        CapStr::<8>(acronym),
        remove_trailing_zeros(speed_change, 2)?.to_arraystring()
    )
    .to_string())
}

#[inline]
pub fn fmt_with_settings(mods: &GameMods) -> Result<String, Error> {
    let mut formatted = Vec::new();
    if mods.is_empty() {
        formatted.push("NM".to_string());
    } else {
        for gamemod in mods {
            let acronym = gamemod.acronym().to_string();
            match gamemod {
                GameMod::DoubleTimeCatch(rate_change_mod) => {
                    if let Some(speed_change) = rate_change_mod.speed_change {
                        formatted.push(format_speed_change(speed_change, &acronym)?);
                    } else {
                        formatted.push(acronym);
                    }
                }
                GameMod::DoubleTimeOsu(rate_change_mod) => {
                    if let Some(speed_change) = rate_change_mod.speed_change {
                        formatted.push(format_speed_change(speed_change, &acronym)?);
                    } else {
                        formatted.push(acronym);
                    }
                }
                GameMod::DoubleTimeTaiko(rate_change_mod) => {
                    if let Some(speed_change) = rate_change_mod.speed_change {
                        formatted.push(format_speed_change(speed_change, &acronym)?);
                    } else {
                        formatted.push(acronym);
                    }
                }
                GameMod::DoubleTimeMania(rate_change_mod) => {
                    if let Some(speed_change) = rate_change_mod.speed_change {
                        formatted.push(format_speed_change(speed_change, &acronym)?);
                    } else {
                        formatted.push(acronym);
                    }
                }
                GameMod::NightcoreOsu(rate_change_mod) => {
                    if let Some(speed_change) = rate_change_mod.speed_change {
                        formatted.push(format_speed_change(speed_change, &acronym)?);
                    } else {
                        formatted.push(acronym);
                    }
                }
                GameMod::NightcoreCatch(rate_change_mod) => {
                    if let Some(speed_change) = rate_change_mod.speed_change {
                        formatted.push(format_speed_change(speed_change, &acronym)?);
                    } else {
                        formatted.push(acronym);
                    }
                }
                GameMod::NightcoreMania(rate_change_mod) => {
                    if let Some(speed_change) = rate_change_mod.speed_change {
                        formatted.push(format_speed_change(speed_change, &acronym)?);
                    } else {
                        formatted.push(acronym);
                    }
                }
                GameMod::NightcoreTaiko(rate_change_mod) => {
                    if let Some(speed_change) = rate_change_mod.speed_change {
                        formatted.push(format_speed_change(speed_change, &acronym)?);
                    } else {
                        formatted.push(acronym);
                    }
                }
                GameMod::HalfTimeOsu(rate_change_mod) => {
                    if let Some(speed_change) = rate_change_mod.speed_change {
                        formatted.push(format_speed_change(speed_change, &acronym)?);
                    } else {
                        formatted.push(acronym);
                    }
                }
                GameMod::HalfTimeTaiko(rate_change_mod) => {
                    if let Some(speed_change) = rate_change_mod.speed_change {
                        formatted.push(format_speed_change(speed_change, &acronym)?);
                    } else {
                        formatted.push(acronym);
                    }
                }
                GameMod::HalfTimeCatch(rate_change_mod) => {
                    if let Some(speed_change) = rate_change_mod.speed_change {
                        formatted.push(format_speed_change(speed_change, &acronym)?);
                    } else {
                        formatted.push(acronym);
                    }
                }
                GameMod::HalfTimeMania(rate_change_mod) => {
                    if let Some(speed_change) = rate_change_mod.speed_change {
                        formatted.push(format_speed_change(speed_change, &acronym)?);
                    } else {
                        formatted.push(acronym);
                    }
                }
                GameMod::DaycoreOsu(rate_change_mod) => {
                    if let Some(speed_change) = rate_change_mod.speed_change {
                        formatted.push(format_speed_change(speed_change, &acronym)?);
                    } else {
                        formatted.push(acronym);
                    }
                }
                GameMod::DaycoreCatch(rate_change_mod) => {
                    if let Some(speed_change) = rate_change_mod.speed_change {
                        formatted.push(format_speed_change(speed_change, &acronym)?);
                    } else {
                        formatted.push(acronym);
                    }
                }
                GameMod::DaycoreTaiko(rate_change_mod) => {
                    if let Some(speed_change) = rate_change_mod.speed_change {
                        formatted.push(format_speed_change(speed_change, &acronym)?);
                    } else {
                        formatted.push(acronym);
                    }
                }
                GameMod::DaycoreMania(rate_change_mod) => {
                    if let Some(speed_change) = rate_change_mod.speed_change {
                        formatted.push(format_speed_change(speed_change, &acronym)?);
                    } else {
                        formatted.push(acronym);
                    }
                }
                GameMod::DifficultyAdjustOsu(difficulty_adjust_mod) => {
                    let mut settings = Vec::new();
                    if let Some(circle_size) = difficulty_adjust_mod.circle_size {
                        settings.push(aformat!("CS{}", remove_trailing_zeros(circle_size, 2)?));
                    }
                    if let Some(overall_difficulty) = difficulty_adjust_mod.overall_difficulty {
                        settings.push(aformat!(
                            "OD{}",
                            remove_trailing_zeros(overall_difficulty, 2)?
                        ));
                    }
                    if let Some(approach_rate) = difficulty_adjust_mod.approach_rate {
                        settings.push(aformat!("AR{}", remove_trailing_zeros(approach_rate, 2)?));
                    }
                    if let Some(drain_rate) = difficulty_adjust_mod.drain_rate {
                        settings.push(aformat!("HP{}", remove_trailing_zeros(drain_rate, 2)?));
                    }
                    if settings.is_empty() {
                        formatted.push(acronym);
                    } else {
                        formatted.push(
                            aformat!(
                                "{} ({})",
                                CapStr::<8>(&acronym),
                                CapStr::<64>(&settings.join(","))
                            )
                            .to_string(),
                        );
                    }
                }
                GameMod::DifficultyAdjustTaiko(difficulty_adjust_mod) => {
                    let mut settings = Vec::new();
                    if let Some(overall_difficulty) = difficulty_adjust_mod.overall_difficulty {
                        settings.push(aformat!(
                            "OD{}",
                            remove_trailing_zeros(overall_difficulty, 2)?
                        ));
                    }
                    if let Some(drain_rate) = difficulty_adjust_mod.drain_rate {
                        settings.push(aformat!("HP{}", remove_trailing_zeros(drain_rate, 2)?));
                    }
                    if settings.is_empty() {
                        formatted.push(acronym);
                    } else {
                        formatted.push(
                            aformat!(
                                "{} ({})",
                                CapStr::<8>(&acronym),
                                CapStr::<64>(&settings.join(","))
                            )
                            .to_string(),
                        );
                    }
                }
                GameMod::DifficultyAdjustCatch(difficulty_adjust_mod) => {
                    let mut settings = Vec::new();
                    if let Some(circle_size) = difficulty_adjust_mod.circle_size {
                        settings.push(aformat!("CS{}", remove_trailing_zeros(circle_size, 2)?));
                    }
                    if let Some(overall_difficulty) = difficulty_adjust_mod.overall_difficulty {
                        settings.push(aformat!(
                            "OD{}",
                            remove_trailing_zeros(overall_difficulty, 2)?
                        ));
                    }
                    if let Some(approach_rate) = difficulty_adjust_mod.approach_rate {
                        settings.push(aformat!("AR{}", remove_trailing_zeros(approach_rate, 2)?));
                    }
                    if let Some(drain_rate) = difficulty_adjust_mod.drain_rate {
                        settings.push(aformat!("HP{}", remove_trailing_zeros(drain_rate, 2)?));
                    }
                    if settings.is_empty() {
                        formatted.push(acronym);
                    } else {
                        formatted.push(
                            aformat!(
                                "{} ({})",
                                CapStr::<8>(&acronym),
                                CapStr::<64>(&settings.join(","))
                            )
                            .to_string(),
                        );
                    }
                }
                GameMod::DifficultyAdjustMania(difficulty_adjust_mod) => {
                    let mut settings = Vec::new();
                    if let Some(overall_difficulty) = difficulty_adjust_mod.overall_difficulty {
                        settings.push(aformat!(
                            "OD{}",
                            remove_trailing_zeros(overall_difficulty, 2)?
                        ));
                    }
                    if let Some(drain_rate) = difficulty_adjust_mod.drain_rate {
                        settings.push(aformat!("HP{}", remove_trailing_zeros(drain_rate, 2)?));
                    }
                    if settings.is_empty() {
                        formatted.push(acronym);
                    } else {
                        formatted.push(
                            aformat!(
                                "{} ({})",
                                CapStr::<8>(&acronym),
                                CapStr::<64>(&settings.join(","))
                            )
                            .to_string(),
                        );
                    }
                }
                _ => formatted.push(acronym.clone()),
            }
        }
    }
    Ok(formatted.join(","))
}

pub fn format_rank_status(status: RankStatus) -> String {
    match status {
        RankStatus::Graveyard => String::from("Graveyard"),
        RankStatus::WIP => String::from("WIP"),
        RankStatus::Pending => String::from("Pending"),
        RankStatus::Ranked => String::from("Ranked"),
        RankStatus::Approved => String::from("Approved"),
        RankStatus::Qualified => String::from("Qualified"),
        RankStatus::Loved => String::from("Loved"),
    }
}

pub fn format_mode_abbreviation(mode: GameMode) -> String {
    match mode {
        GameMode::Osu => String::from("o!s"),
        GameMode::Taiko => String::from("o!t"),
        GameMode::Catch => String::from("o!c"),
        GameMode::Mania => String::from("o!m"),
    }
}

pub fn format_footer(
    score: &Score,
    beatmap: &Beatmap,
    pp: &CalculateResults,
) -> Result<String, Error> {
    match pp.max_pp {
        Some(max_pp) => {
            if (score.grade == Grade::F || !score.passed) && score.mode != GameMode::Catch {
                let beatmap_objects = f64::from(
                    beatmap.count_spinners + beatmap.count_circles + beatmap.count_sliders,
                );
                if ((pp.pp / max_pp) * 100.0) < 99.0 {
                    Ok(aformat!(
                        "Potential: {}pp, completed {}%({}★)",
                        remove_trailing_zeros(max_pp, 2)?.to_arraystring(),
                        remove_trailing_zeros(
                            (f64::from(score.total_hits()) / beatmap_objects) * 100.0,
                            2
                        )?
                        .to_arraystring(),
                        remove_trailing_zeros(pp.partial_stars, 2)?.to_arraystring()
                    )
                    .to_string())
                } else {
                    Ok(aformat!(
                        "Completion rate: {}%({}★)",
                        remove_trailing_zeros(
                            (f64::from(score.total_hits()) / beatmap_objects) * 100.0,
                            2
                        )?
                        .to_arraystring(),
                        remove_trailing_zeros(pp.partial_stars, 2)?.to_arraystring()
                    )
                    .to_string())
                }
            } else if ((pp.pp / max_pp) * 100.0) < 99.0 {
                Ok(aformat!(
                    "Potential: {}pp, +{}pp",
                    remove_trailing_zeros(max_pp, 2)?.to_arraystring(),
                    remove_trailing_zeros(max_pp - pp.pp, 2)?.to_arraystring()
                )
                .to_string())
            } else {
                Ok(String::new())
            }
        }
        _ => {
            if (score.grade == Grade::F || !score.passed) && score.mode != GameMode::Catch {
                let beatmap_objects = f64::from(
                    beatmap.count_spinners + beatmap.count_circles + beatmap.count_sliders,
                );
                Ok(aformat!(
                    "Completion rate: {}%({}★)",
                    remove_trailing_zeros(
                        (f64::from(score.total_hits()) / beatmap_objects) * 100.0,
                        2
                    )?
                    .to_arraystring(),
                    remove_trailing_zeros(pp.partial_stars, 2)?.to_arraystring()
                )
                .to_string())
            } else {
                Ok(String::new())
            }
        }
    }
}

pub fn format_diff(new: &OsuUser, old: &OsuUser, mode: GameMode) -> Result<String, Error> {
    let pp_diff = get_stat_diff(old, new, &DiffTypes::Pp);
    let country_diff = -get_stat_diff(old, new, &DiffTypes::CountryRank);
    let global_diff = -get_stat_diff(old, new, &DiffTypes::GlobalRank);
    let acc_diff = get_stat_diff(old, new, &DiffTypes::Acc);
    let score_diff = get_stat_diff(old, new, &DiffTypes::Score);

    let formatted_pp_diff = if pp_diff == 0.0 {
        String::new()
    } else {
        format!(" {:+}pp", remove_trailing_zeros(pp_diff, 2)?)
    };

    let formatted_global_diff = if global_diff == 0.0 {
        ArrayString::<34>::new()
    } else if global_diff > 0.0 {
        aformat!(
            " +{}",
            CapStr::<32>(&(global_diff as i64).to_formatted_string(&Locale::en))
        )
    } else {
        aformat!(
            " {}",
            CapStr::<33>(&(global_diff as i64).to_formatted_string(&Locale::en))
        )
    };

    let formatted_country_diff = if country_diff == 0.0 {
        ArrayString::<34>::new()
    } else if country_diff > 0.0 {
        aformat!(
            " +{}",
            CapStr::<32>(&(country_diff as i64).to_formatted_string(&Locale::en))
        )
    } else {
        aformat!(
            " {}",
            CapStr::<33>(&(country_diff as i64).to_formatted_string(&Locale::en))
        )
    };

    let formatted_acc_diff = if acc_diff == 0.0 {
        String::new()
    } else {
        format!(" {:+}%", remove_trailing_zeros(acc_diff, 2)?)
    };

    let formatted_score_diff = if score_diff == 0.0 {
        ArrayString::<34>::new()
    } else if score_diff > 0.0 {
        aformat!(
            " +{}",
            CapStr::<32>(&(score_diff as i64).to_formatted_string(&Locale::en))
        )
    } else {
        aformat!(
            " {}",
            CapStr::<33>(&(score_diff as i64).to_formatted_string(&Locale::en))
        )
    };

    let acc_emoji = if acc_diff > 0.0 {
        "\u{1f4c8}"
    } else if acc_diff < 0.0 {
        "\u{1f4c9}"
    } else {
        "\u{1f3af}"
    };

    Ok(aformat!(
        "`{} {}pp{}` \u{1F30D}`#{}{}` :flag_{}:`#{}{}`\n{}`{}%{}` \u{1f522}`{}{}`",
        CapStr::<12>(&format_mode_abbreviation(mode)),
        remove_trailing_zeros(new.pp, 2)?.to_arraystring(),
        CapStr::<32>(&formatted_pp_diff),
        CapStr::<32>(&new.global_rank.to_formatted_string(&Locale::en)),
        formatted_global_diff,
        CapStr::<8>(&new.country_code.to_lowercase()),
        CapStr::<32>(&new.country_rank.to_formatted_string(&Locale::en)),
        formatted_country_diff,
        CapStr::<12>(acc_emoji),
        remove_trailing_zeros(new.accuracy, 2)?.to_arraystring(),
        CapStr::<32>(&formatted_acc_diff),
        CapStr::<32>(&new.ranked_score.to_formatted_string(&Locale::en)),
        formatted_score_diff
    )
    .to_string())
}

pub async fn format_missing_user_string(ctx: Context<'_>, user: &User) -> Result<String, Error> {
    Ok(aformat!("No osu! profile assigned to **{}**! Please assign a profile using **{}osu link <username>**", 
               CapStr::<128>(&user.name),
               CapStr::<4>(&crate::utils::db::prefix::get_guild_prefix(ctx.into()).await?.ok_or("Failed to get guild prefix in format_missing_user function")?)).to_string())
}

pub fn format_beatmap_link(
    beatmap_id: Option<i64>,
    beatmapset_id: i64,
    mode: Option<&str>,
) -> String {
    if let (Some(beatmap_id), Some(mode)) = (beatmap_id, mode) {
        aformat!(
            "https://osu.ppy.sh/beatmapsets/{}#{}/{}",
            beatmapset_id.to_arraystring(),
            CapStr::<16>(mode),
            beatmap_id.to_arraystring()
        )
        .to_string()
    } else {
        aformat!(
            "https://osu.ppy.sh/beatmapsets/{}",
            beatmapset_id.to_arraystring()
        )
        .to_string()
    }
}

pub fn format_user_link(user_id: i64) -> String {
    aformat!("https://osu.ppy.sh/users/{}", user_id.to_arraystring()).to_string()
}
