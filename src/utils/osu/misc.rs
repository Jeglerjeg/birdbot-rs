use crate::models::osu_users::OsuUser;
use crate::plugins::osu::SortChoices;
use crate::utils::db::{linked_osu_profiles, osu_notifications, osu_users};
use crate::utils::osu::misc_format::format_missing_user_string;
use crate::Error;
use diesel::PgConnection;
use poise::serenity_prelude::{Context, Presence, UserId};
use rosu_v2::model::GameMode;
use rosu_v2::prelude::{Score, User};
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
        "osu" | "standard" | "std" | "osu!" | "0" => Some(GameMode::Osu),
        "taiko" | "osu!taiko" | "1" => Some(GameMode::Taiko),
        "mania" | "keys" | "osu!mania" | "3" => Some(GameMode::Mania),
        "catch" | "ctb" | "fruits" | "osu!catch" | "2" => Some(GameMode::Catch),
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

pub fn count_score_pages(score_count: usize, scores_per_page: usize) -> usize {
    (score_count + scores_per_page - 1) / scores_per_page
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

pub fn is_playing(ctx: &Context, user_id: UserId, home_guild: i64) -> bool {
    let mut presence: Option<Presence> = None;
    let fetched_guild = ctx.cache.guild(home_guild as u64);
    if let Some(guild_ref) = fetched_guild {
        let guild = Arc::from(guild_ref.clone());
        if guild.members.contains_key(&user_id) {
            let presences = &guild.presences;
            presence = presences.get(&user_id).cloned();
        } else {
            for guild in ctx.cache.guilds() {
                let cached_guild = Arc::from(guild.to_guild_cached(ctx).unwrap());
                if let Some(_member) = ctx.cache.member(guild, user_id) {
                    presence = cached_guild
                        .clone()
                        .presences
                        .clone()
                        .get(&user_id)
                        .cloned();
                }
            }
        }
    } else {
        for guild in ctx.cache.guilds() {
            if let Some(_member) = ctx.cache.member(guild, user_id) {
                presence = guild
                    .to_guild_cached(&ctx.cache)
                    .unwrap()
                    .presences
                    .get(&user_id)
                    .cloned();
            }
        }
    }

    if let Some(presence) = presence {
        for activity in presence.activities {
            if activity.name.to_lowercase().contains("osu!") {
                return true;
            }
        }
    }

    false
}

pub fn sort_scores(mut scores: Vec<(Score, usize)>, sort_by: &SortChoices) -> Vec<(Score, usize)> {
    match sort_by {
        SortChoices::Recent => {
            scores.sort_by(|a, b| b.0.ended_at.cmp(&a.0.ended_at));
        }
        SortChoices::Oldest => scores.sort_by(|a, b| a.0.ended_at.cmp(&b.0.ended_at)),
        SortChoices::Accuracy => {
            scores.sort_by(|a, b| b.0.accuracy.total_cmp(&a.0.accuracy));
        }
        SortChoices::Combo => scores.sort_by(|a, b| b.0.max_combo.cmp(&a.0.max_combo)),
        SortChoices::Score => scores.sort_by(|a, b| b.0.score.cmp(&a.0.score)),
        SortChoices::PP => {
            scores.sort_by(|a, b| b.0.pp.unwrap_or(0.0).total_cmp(&a.0.pp.unwrap_or(0.0)));
        }
    }
    scores
}

pub async fn get_user(
    ctx: crate::Context<'_>,
    discord_user: &poise::serenity_prelude::User,
    user: Option<String>,
    connection: &mut PgConnection,
) -> Result<Option<User>, Error> {
    if let Some(user) = user {
        if let Ok(user) = ctx.data().osu_client.user(user).await {
            Ok(Some(user))
        } else {
            ctx.say("Could not find user.").await?;
            Ok(None)
        }
    } else {
        let linked_profile = linked_osu_profiles::read(connection, discord_user.id.0.get() as i64);
        if let Ok(linked_profile) = linked_profile {
            if let Ok(user) = ctx
                .data()
                .osu_client
                .user(linked_profile.osu_id as u32)
                .mode(gamemode_from_string(&linked_profile.mode).unwrap())
                .await
            {
                Ok(Some(user))
            } else {
                ctx.say("Could not find user.").await?;
                Ok(None)
            }
        } else {
            ctx.say(format_missing_user_string(ctx, discord_user).await?)
                .await?;
            Ok(None)
        }
    }
}
