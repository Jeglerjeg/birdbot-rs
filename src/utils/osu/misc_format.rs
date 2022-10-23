use crate::models::beatmaps::Beatmap;
use crate::models::osu_users::OsuUser;
use crate::utils::misc::remove_trailing_zeros;
use crate::utils::osu::misc::{get_stat_diff, DiffTypes};
use crate::utils::osu::pp::CalculateResults;
use crate::Context;
use num_format::{Locale, ToFormattedString};
use rosu_v2::model::beatmap::RankStatus;
use rosu_v2::model::GameMode;
use rosu_v2::prelude::Score;
use serenity::model::user::User;

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
        GameMode::Osu => String::from("osu!"),
        GameMode::Taiko => String::from("o!t"),
        GameMode::Catch => String::from("o!c"),
        GameMode::Mania => String::from("o!m"),
    }
}

pub fn format_potential_string(pp: &CalculateResults) -> String {
    match pp.max_pp {
        Some(max_pp) => {
            if ((pp.pp / max_pp) * 100.0) < 99.0 {
                format!(
                    "Potential: {}pp, {:+}pp",
                    remove_trailing_zeros(max_pp, 2),
                    remove_trailing_zeros(max_pp - pp.pp, 2)
                )
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

pub fn format_completion_rate(score: &Score, beatmap: &Beatmap, pp: &CalculateResults) -> String {
    let beatmap_objects =
        f64::from(beatmap.count_spinners + beatmap.count_circles + beatmap.count_sliders);
    format!(
        "Completion rate: {}%({}★)",
        remove_trailing_zeros((f64::from(score.total_hits()) / beatmap_objects) * 100.0, 2),
        remove_trailing_zeros(pp.partial_stars, 2)
    )
}

pub fn format_diff(new: &OsuUser, old: &OsuUser, mode: GameMode) -> String {
    let pp_diff = get_stat_diff(old, new, &DiffTypes::Pp);
    let country_diff = get_stat_diff(old, new, &DiffTypes::CountryRank);
    let global_diff = get_stat_diff(old, new, &DiffTypes::GlobalRank);
    let acc_diff = get_stat_diff(old, new, &DiffTypes::Acc);
    let score_diff = get_stat_diff(old, new, &DiffTypes::Score);

    let formatted_pp_diff = if pp_diff == 0.0 {
        String::new()
    } else {
        format!(" {:+}", remove_trailing_zeros(pp_diff, 2))
    };

    let formatted_global_diff = if global_diff == 0.0 {
        String::new()
    } else if global_diff < 0.0 {
        format!(" +{}", remove_trailing_zeros(global_diff, 2))
    } else {
        format!(" -{}", remove_trailing_zeros(global_diff, 2))
    };

    let formatted_country_diff = if country_diff == 0.0 {
        String::new()
    } else if country_diff < 0.0 {
        format!(" +{}", remove_trailing_zeros(country_diff, 2))
    } else {
        format!(" -{}", remove_trailing_zeros(country_diff, 2))
    };

    let formatted_acc_diff = if acc_diff == 0.0 {
        String::new()
    } else {
        format!(" {:+}", remove_trailing_zeros(acc_diff, 2))
    };

    let formatted_score_diff = if score_diff == 0.0 {
        String::new()
    } else {
        format!(" {:+}", remove_trailing_zeros(score_diff, 2))
    };

    let acc_emoji = if acc_diff > 0.0 {
        "\u{1f4c8}"
    } else if acc_diff < 0.0 {
        "\u{1f4c9}"
    } else {
        "\u{1f3af}"
    };

    format!(
        "\u{2139}`{} {}{}` \u{1F30D}`{}{}` :flag_{}:`{}{}`\n{}`{}{}` \u{1f522}`{}{}`",
        format_mode_abbreviation(mode),
        remove_trailing_zeros(new.pp, 2),
        formatted_pp_diff,
        new.global_rank.to_formatted_string(&Locale::en),
        formatted_global_diff,
        new.country_code.to_lowercase(),
        new.country_rank.to_formatted_string(&Locale::en),
        formatted_country_diff,
        acc_emoji,
        remove_trailing_zeros(new.accuracy, 2),
        formatted_acc_diff,
        new.ranked_score.to_formatted_string(&Locale::en),
        formatted_score_diff
    )
}

pub async fn format_missing_user_string(ctx: Context<'_>, user: &User) -> String {
    format!("No osu! profile assigned to **{}**! Please assign a profile using **{}osu link <username>**", user.name, crate::utils::db::prefix::get_guild_prefix(ctx.into()).await.unwrap().unwrap())
}

pub fn format_beatmap_link(beatmap_id: i64, beatmapset_id: i64, mode: &str) -> String {
    format!("https://osu.ppy.sh/beatmapsets/{beatmapset_id}#{mode}/{beatmap_id}")
}

pub fn format_user_link(user_id: i64) -> String {
    format!("https://osu.ppy.sh/users/{}", user_id)
}
