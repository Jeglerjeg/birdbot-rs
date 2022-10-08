use crate::models::beatmaps::Beatmap;
use crate::utils::misc::remove_trailing_zeros;
use crate::utils::osu::pp::CalculateResults;
use crate::Context;
use rosu_v2::model::beatmap::RankStatus;
use rosu_v2::prelude::Score;

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
        (beatmap.count_spinners + beatmap.count_circles + beatmap.count_sliders) as f64;
    format!(
        "Completion rate: {}%({}★)",
        remove_trailing_zeros((score.total_hits() as f64 / beatmap_objects) * 100.0, 2),
        remove_trailing_zeros(pp.partial_stars, 2)
    )
}

pub async fn format_missing_user_string(ctx: Context<'_>) -> String {
    format!("No osu! profile assigned to **{}**! Please assign a profile using **{}osu link <username>**", ctx.author().name, crate::utils::db::prefix::get_guild_prefix(ctx.into()).await.unwrap().unwrap())
}

pub fn format_beatmap_link(beatmap_id: i64, beatmapset_id: i64, mode: &str) -> String {
    format!("https://osu.ppy.sh/beatmapsets/{beatmapset_id}#{mode}/{beatmap_id}")
}

pub fn format_user_link(user_id: u32) -> String {
    format!("https://osu.ppy.sh/users/{}", user_id)
}
