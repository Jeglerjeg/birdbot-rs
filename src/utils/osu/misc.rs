use crate::Error;
use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::models::osu_files::OsuFile;
use crate::models::osu_users::{NewOsuUser, OsuUser};
use crate::plugins::osu::{GameModeChoices, SortChoices};
use crate::utils::db::{linked_osu_profiles, osu_notifications, osu_users};
use crate::utils::osu::caching::get_beatmap;
use crate::utils::osu::calculate;
use crate::utils::osu::pp::CalculateResults;
use crate::utils::osu::regex::{BeatmapInfo, get_beatmap_info};
use diesel_async::AsyncPgConnection;
use par_stream::ParStreamExt;
use poise::futures_util::StreamExt;
use poise::serenity_prelude::{Cache, GuildId, Message, Presence, User, UserId};
use rosu_v2::model::GameMode;
use rosu_v2::prelude::{Score, ScoreStatistics, UserExtended};
use serde::{Deserialize, Serialize};
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
            let numerator = 300 * score.statistics.great
                + 100 * score.statistics.good
                + 50 * score.statistics.meh
                + 150 * score.statistics.slider_tail_hit
                + 30 * score.statistics.large_tick_hit;

            let denominator = 300 * score.statistics.great
                + 300 * score.statistics.good
                + 300 * score.statistics.meh
                + 300 * score.statistics.miss
                + 150 * score.statistics.slider_tail_hit
                + 30 * score.statistics.large_tick_hit;

            Some(f64::from(numerator) / f64::from(denominator))
        }
        _ => None,
    }
}

pub fn is_perfect(statistics: &ScoreStatistics) -> bool {
    if statistics.miss > 0 || statistics.large_tick_miss > 0 || statistics.combo_break > 0 {
        return false;
    }
    true
}

pub fn count_score_pages(score_count: usize, scores_per_page: usize) -> usize {
    score_count.div_ceil(scores_per_page)
}

pub async fn wipe_profile_data(db: &mut AsyncPgConnection, user_id: i64) -> Result<(), Error> {
    if osu_users::read(db, user_id).await.is_ok() {
        osu_users::delete(db, user_id).await?;
    }

    if osu_notifications::read(db, user_id).await.is_ok() {
        osu_notifications::delete(db, user_id).await?;
    }

    Ok(())
}

pub fn is_playing(cache: &Cache, user_id: UserId, home_guild: i64) -> Result<bool, Error> {
    let mut presence: Option<Presence> = None;
    if let Some(guild_ref) = cache.guild(GuildId::new(u64::try_from(home_guild)?)) {
        if guild_ref.members.contains_key(&user_id) {
            let presences = &guild_ref.presences;
            presence = presences.get(&user_id).cloned();
        }
    }

    if presence.is_none() {
        for guild in cache.guilds() {
            if cache
                .guild(guild)
                .ok_or("Failed to get guild from cache")?
                .members
                .contains_key(&user_id)
            {
                presence = guild
                    .to_guild_cached(cache)
                    .ok_or("Failed to get user presences in is_playing function")?
                    .presences
                    .get(&user_id)
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

#[derive(Deserialize, Serialize)]
struct RespektiveUser {
    rank: u32,
    user_id: u32,
}

pub async fn get_score_rank(user_id: u32, mode: GameMode) -> Result<u32, Error> {
    let url = format!("https://score.respektive.pw/u/{user_id}?m={}", mode as u8);
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?.bytes().await?;
    let users: Vec<RespektiveUser> = serde_json::from_slice(&response)?;
    Ok(users.first().ok_or("Failed to get respektive user")?.rank)
}

pub fn sort_scores(
    mut scores: Vec<(Score, usize, Beatmap, Beatmapset, CalculateResults)>,
    sort_by: &SortChoices,
) -> Vec<(Score, usize, Beatmap, Beatmapset, CalculateResults)> {
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
            scores.sort_by(|a, b| {
                b.0.pp
                    .unwrap_or(b.4.pp as f32)
                    .total_cmp(&a.0.pp.unwrap_or(a.4.pp as f32))
            });
        }
        SortChoices::Length => {
            scores.sort_by(|a, b| b.2.drain.cmp(&a.2.drain));
        }
        SortChoices::Misses => {
            scores.sort_by(|a, b| b.0.statistics.miss.cmp(&a.0.statistics.miss));
        }
        SortChoices::Stars => {
            scores.sort_by(|a, b| b.4.total_stars.total_cmp(&a.4.total_stars));
        }
        SortChoices::Bpm => {
            scores
                .sort_by(|a, b| (b.2.bpm * b.4.clock_rate).total_cmp(&(a.2.bpm * a.4.clock_rate)));
        }
    }
    scores
}

pub async fn set_up_score_list(
    ctx: &crate::Context<'_>,
    connection: &mut AsyncPgConnection,
    scores: Vec<Score>,
) -> Result<Vec<(Score, usize, Beatmap, Beatmapset, CalculateResults)>, Error> {
    let mut score_list: Vec<(Score, usize, Beatmap, Beatmapset, CalculateResults)> =
        Vec::with_capacity(100);
    let typing = ctx
        .channel_id()
        .start_typing(ctx.serenity_context().http.clone());
    let mut process_list: Vec<(Score, (Beatmap, Beatmapset, OsuFile))> = Vec::with_capacity(100);
    for score in scores {
        let beatmap = get_beatmap(connection, ctx.data().osu_client.clone(), score.map_id).await?;
        process_list.push((score, beatmap));
    }
    let mut stream = tokio_stream::iter(process_list).par_then(None, move |score| async move {
        let calculated = calculate::calculate(
            Some(&score.0),
            &score.1.0,
            &score.1.2,
            calculate_potential_acc(&score.0),
        )?;

        Ok::<(Score, (Beatmap, Beatmapset, OsuFile), CalculateResults), Error>((
            score.0, score.1, calculated,
        ))
    });
    let mut pos = 0;
    while let Some(value) = stream.next().await {
        pos += 1;
        let value = value?;
        score_list.push((value.0, pos, value.1.0, value.1.1, value.2));
    }
    typing.stop();

    Ok(score_list)
}

pub async fn get_user_by_username(
    ctx: crate::Context<'_>,
    user: &str,
    mode: Option<GameModeChoices>,
) -> Result<Option<UserExtended>, Error> {
    if let Some(mode) = mode {
        let gamemode: GameMode = mode.into();
        if let Ok(mut user) = ctx.data().osu_client.user(user).mode(gamemode).await {
            user.mode = gamemode;
            Ok(Some(user))
        } else {
            ctx.say("Could not find user.").await?;
            Ok(None)
        }
    } else if let Ok(user) = ctx.data().osu_client.user(user).await {
        Ok(Some(user))
    } else {
        ctx.say("Could not find user.").await?;
        Ok(None)
    }
}

pub async fn get_user(
    ctx: crate::Context<'_>,
    discord_user: &User,
    user: Option<String>,
    connection: &mut AsyncPgConnection,
    mode: Option<GameModeChoices>,
) -> Result<Option<UserExtended>, Error> {
    if let Some(user) = user {
        Ok(get_user_by_username(ctx, &user, mode).await?)
    } else {
        let linked_profile =
            linked_osu_profiles::read(connection, i64::try_from(discord_user.id.get())?).await;
        if let Ok(linked_profile) = linked_profile {
            let mode: GameMode = if let Some(mode) = mode {
                mode.into()
            } else {
                gamemode_from_string(&linked_profile.mode)
                    .ok_or("Failed to parse gamemode from string in get_user function")?
            };

            let user = ctx
                .data()
                .osu_client
                .user(u32::try_from(linked_profile.osu_id)?)
                .mode(mode)
                .await;

            if let Ok(mut user) = user {
                user.mode = mode;
                Ok(Some(user))
            } else {
                ctx.say("Could not find user.").await?;
                Ok(None)
            }
        } else {
            Ok(get_user_by_username(ctx, &discord_user.name, mode).await?)
        }
    }
}

pub fn get_osu_user(
    cache: &Cache,
    user_id: UserId,
    home_guild: u64,
) -> Result<Option<User>, Error> {
    let mut user: Option<User> = None;
    if let Some(guild_ref) = cache.guild(GuildId::new(home_guild)) {
        if let Some(guild_user) = guild_ref.members.get(&user_id) {
            user = Some(guild_user.user.clone());
        }
    }

    if user.is_none() {
        for guild in cache.guilds() {
            if let Some(guild_user) = cache
                .guild(guild)
                .ok_or("Failed to get guild from cache")?
                .members
                .get(&user_id)
            {
                user = Some(guild_user.user.clone());
            }
        }
    }

    Ok(user)
}

pub async fn find_beatmap_link(messages: Vec<Message>) -> Result<Option<BeatmapInfo>, Error> {
    for message in messages {
        let mut to_search = message.content.to_string();
        for embed in message.embeds {
            if let Some(description) = embed.description {
                to_search.push_str(&description);
            }

            if let Some(title) = embed.title {
                to_search.push_str(&title);
            }

            if let Some(url) = embed.url {
                to_search.push_str(&url);
            }

            if let Some(footer) = embed.footer {
                to_search.push_str(&footer.text);
            }
        }
        let beatmap_info = get_beatmap_info(&to_search)?;

        if beatmap_info.beatmap_id.is_some() {
            return Ok(Some(beatmap_info));
        }
    }
    Ok(None)
}

pub fn get_score_position(score: &Score, mut score_list: Vec<Score>) -> Result<usize, Error> {
    let mut found_index = None;
    for (i, list_score) in score_list.iter().enumerate() {
        if score.id == list_score.id {
            found_index = Some(i + 1);
            break;
        }
    }

    if let Some(found_index) = found_index {
        return Ok(found_index);
    }

    score_list.push(score.clone());
    score_list.sort_by(|a, b| b.pp.unwrap_or(0.0).total_cmp(&a.pp.unwrap_or(0.0)));
    Ok(score_list
        .iter()
        .position(|x| x.id == score.id)
        .ok_or("Failed to find score position")?
        + 1)
}

pub async fn add_profile_data(
    osu_client: Arc<rosu_v2::Osu>,
    profile_id: u32,
    mode: GameMode,
    db: &mut AsyncPgConnection,
) -> Result<OsuUser, Error> {
    let osu_user = osu_client.user(profile_id).mode(mode).await?;

    let mut top_100 = osu_client
        .user_scores(profile_id)
        .best()
        .mode(mode)
        .limit(100)
        .await?;

    top_100.sort_by(|a, b| b.pp.unwrap_or(0.0).total_cmp(&a.pp.unwrap_or(0.0)));

    let mut new_user = NewOsuUser::try_from(osu_user)?;

    new_user.min_pp = f64::from(top_100[top_100.len() - 1].pp.unwrap_or(0.0));

    osu_users::create(db, &new_user).await
}
