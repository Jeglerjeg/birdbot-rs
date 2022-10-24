use crate::models::osu_users::OsuUser;
use crate::utils::db::{osu_notifications, osu_users};
use crate::Error;
use diesel::PgConnection;
use poise::serenity_prelude;
use rosu_v2::model::GameMode;
use rosu_v2::prelude::Score;
use serenity_prelude::{Context, Presence, User};
use std::sync::Arc;

pub enum DiffTypes {
    Pp,
    Acc,
    GlobalRank,
    CountryRank,
    Score,
}

pub fn get_stat_diff(old: &OsuUser, new: &OsuUser, diff_type: &DiffTypes) -> f64 {
    let old_value: f64;
    let new_value: f64;
    match diff_type {
        DiffTypes::Pp => {
            old_value = old.pp;
            new_value = new.pp;
        }
        DiffTypes::Acc => {
            old_value = old.accuracy;
            new_value = new.accuracy;
        }
        DiffTypes::GlobalRank => {
            old_value = f64::from(old.global_rank);
            new_value = f64::from(new.global_rank);
        }
        DiffTypes::CountryRank => {
            old_value = f64::from(old.country_rank);
            new_value = f64::from(new.country_rank);
        }
        DiffTypes::Score => {
            old_value = old.ranked_score as f64;
            new_value = new.ranked_score as f64;
        }
    }

    new_value - old_value
}

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

pub fn count_score_pages(scores: &[(Score, usize)], scores_per_page: usize) -> usize {
    (scores.len() + scores_per_page - 1) / scores_per_page
}

pub fn wipe_profile_data(db: &mut PgConnection, user_id: i64) -> Result<(), Error> {
    if osu_users::read(db, user_id).is_ok() {
        osu_users::delete(db, user_id)?;
    }

    if osu_notifications::read(db, user_id).is_ok() {
        osu_notifications::delete(db, user_id)?;
    }

    Ok(())
}

pub async fn is_playing(ctx: &Context, user: User, home_guild: i64) -> Result<bool, Error> {
    let mut presence: Option<Presence> = None;
    let fetched_guild = ctx.cache.guild(home_guild as u64);
    if let Some(guild_ref) = fetched_guild {
        let guild = Arc::from(guild_ref.clone());
        if guild.members.contains_key(&user.id) {
            let presences = &guild.clone().presences;
            presence = presences.get(&user.id).cloned();
        } else {
            for guild in ctx.cache.guilds() {
                let cached_guild = Arc::from(guild.to_guild_cached(ctx).unwrap());
                if let Some(_member) = ctx.cache.member(guild, user.id) {
                    presence = cached_guild
                        .clone()
                        .presences
                        .clone()
                        .get(&user.id)
                        .cloned();
                }
            }
        }
    } else {
        for guild in ctx.cache.guilds() {
            if let Some(_member) = ctx.cache.member(guild, user.id) {
                presence = guild
                    .to_guild_cached(&ctx.cache)
                    .unwrap()
                    .presences
                    .get(&user.id)
                    .cloned();
            }
        }
    }

    if let Some(presence) = presence {
        for activity in presence.activities {
            if activity.name.to_lowercase().contains("osu!") {
                return Ok(true);
            }
        }
    }

    Ok(false)
}
