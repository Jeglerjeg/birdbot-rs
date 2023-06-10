use crate::models::linked_osu_profiles::NewLinkedOsuProfile;
use crate::utils::db::{linked_osu_profiles, osu_guild_channels, osu_notifications, osu_users};
use crate::utils::osu::misc::{
    find_beatmap_link, get_user, is_playing, sort_scores, wipe_profile_data,
};
use crate::utils::osu::misc_format::format_missing_user_string;
use chrono::Utc;
use poise::serenity_prelude::model::colour::colours::roles::BLUE;
use poise::serenity_prelude::{CacheHttp, Colour, CreateEmbed, CreateEmbedAuthor, GuildChannel};
use poise::CreateReply;
use rosu_v2::model::GameMode;
use rosu_v2::prelude::Score;

use crate::models::osu_guild_channels::NewOsuGuildChannel;
use crate::models::osu_notifications::NewOsuNotification;
use crate::{Context, Error};

use crate::utils::osu::embeds::{send_score_embed, send_scores_embed};
use crate::utils::osu::regex::{get_beatmap_info, BeatmapInfo};

/// Display information about your osu! user.
#[poise::command(
    prefix_command,
    slash_command,
    category = "osu!",
    subcommands(
        "link",
        "score",
        "scores",
        "unlink",
        "mode",
        "recent",
        "recent_best",
        "pins",
        "firsts",
        "top",
        "score_notifications",
        "map_notifications",
        "delete_guild_config",
        "debug"
    )
)]
pub async fn osu(ctx: Context<'_>) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get().await?;
    let profile = linked_osu_profiles::read(connection, ctx.author().id.0.get() as i64).await;
    match profile {
        Ok(profile) => {
            let color: Colour;
            if let Some(guild) = ctx.guild() {
                if let Some(member) = ctx
                    .cache()
                    .ok_or("Failed to get discord cache in osu command")?
                    .member(guild.id, ctx.author().id)
                {
                    color = member.colour(ctx.discord()).unwrap_or(BLUE);
                } else {
                    color = BLUE;
                }
            } else {
                color = BLUE;
            };

            let colour_formatted =
                format!("%23{:02x}{:02x}{:02x}", color.r(), color.g(), color.b());

            let darkheader = if (f32::from(color.r()) * 0.299
                + f32::from(color.g()) * 0.587
                + f32::from(color.b()) * 0.144)
                > 186.0
            {
                "&darkheader"
            } else {
                ""
            };

            let mode = match profile.mode.as_str() {
                "osu" => 0,
                "taiko" => 1,
                "fruits" => 2,
                "mania" => 3,
                _ => 10,
            };

            let author = CreateEmbedAuthor::new(&ctx.author().name).icon_url(ctx.author().face());

            let embed = CreateEmbed::new()
                .image(format!("https://osusig.lolicon.app/sig.php?colour={}&uname={}&pp=0&countryrank=&xpbar=&mode={}&date={}{}",
                               colour_formatted, profile.osu_id, mode, Utc::now().timestamp(), darkheader)).color(color).author(author);

            let builder = CreateReply::default().embed(embed);

            ctx.send(builder).await?;
        }
        Err(_) => {
            ctx.say(format_missing_user_string(ctx, ctx.author()).await?)
                .await?;
        }
    }

    Ok(())
}

/// Link an osu! profile.
#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    category = "osu!",
    aliases("set")
)]
pub async fn link(
    ctx: Context<'_>,
    #[rest]
    #[description = "osu! username to link to"]
    username: String,
) -> Result<(), Error> {
    let user = ctx.data().osu_client.user(username).await?;
    let connection = &mut ctx.data().db_pool.get().await?;

    if let Ok(profile) = linked_osu_profiles::read(connection, ctx.author().id.0.get() as i64).await
    {
        linked_osu_profiles::delete(connection, profile.id).await?;
        wipe_profile_data(connection, profile.osu_id).await?;
    }

    let query_item = NewLinkedOsuProfile {
        id: ctx.author().id.0.get() as i64,
        osu_id: i64::from(user.user_id),
        home_guild: ctx
            .guild_id()
            .ok_or("Failed to get guild ID in link command")?
            .0
            .get() as i64,
        mode: user.mode.to_string(),
    };

    let notification_item = NewOsuNotification {
        id: i64::from(user.user_id),
        last_pp: Utc::now(),
        last_event: Utc::now(),
    };
    osu_notifications::create(connection, &notification_item).await?;

    linked_osu_profiles::create(connection, &query_item).await?;

    ctx.say(format!(
        "Set your osu! profile to `{}`.",
        user.username.as_str()
    ))
    .await?;

    Ok(())
}

/// Unlink your osu! profile.
#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    category = "osu!",
    aliases("unset")
)]
pub async fn unlink(ctx: Context<'_>) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get().await?;
    let profile = linked_osu_profiles::read(connection, ctx.author().id.0.get() as i64).await;

    match profile {
        Ok(profile) => {
            linked_osu_profiles::delete(connection, profile.id).await?;
            wipe_profile_data(connection, profile.osu_id).await?;
            ctx.say("Unlinked your profile.").await?;
        }
        Err(_) => {
            ctx.say(format_missing_user_string(ctx, ctx.author()).await?)
                .await?;
        }
    };

    Ok(())
}

#[derive(poise::ChoiceParameter)]
pub enum GameModeChoices {
    #[name = "Standard"]
    #[name = "osu"]
    #[name = "osu!"]
    #[name = "std"]
    #[name = "osu!standard"]
    Standard,
    #[name = "Mania"]
    #[name = "Keys"]
    #[name = "osu!mania"]
    Mania,
    #[name = "Catch"]
    #[name = "ctb"]
    #[name = "fruits"]
    #[name = "osu!catch"]
    Catch,
    #[name = "Taiko"]
    #[name = "osu!taiko"]
    #[name = "drums"]
    Taiko,
}

/// Changed your osu! mode.
#[poise::command(
    prefix_command,
    slash_command,
    category = "osu!",
    aliases("mode", "m", "track")
)]
pub async fn mode(
    ctx: Context<'_>,
    #[description = "Gamemode to switch to."] new_mode: GameModeChoices,
) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get().await?;
    let profile = linked_osu_profiles::read(connection, ctx.author().id.0.get() as i64).await;

    let mode = match new_mode {
        GameModeChoices::Standard => GameMode::Osu,
        GameModeChoices::Taiko => GameMode::Taiko,
        GameModeChoices::Catch => GameMode::Catch,
        GameModeChoices::Mania => GameMode::Mania,
    };

    match profile {
        Ok(profile) => {
            let query_item = NewLinkedOsuProfile {
                id: profile.id,
                osu_id: profile.osu_id,
                home_guild: profile.home_guild,
                mode: mode.to_string(),
            };

            linked_osu_profiles::update(connection, profile.id, &query_item).await?;
            wipe_profile_data(connection, profile.osu_id).await?;

            ctx.say(format!("Updated your osu! mode to {mode}."))
                .await?;
        }
        Err(_) => {
            ctx.say(format_missing_user_string(ctx, ctx.author()).await?)
                .await?;
        }
    }

    Ok(())
}

/// Display your score on a beatmap.
#[poise::command(prefix_command, slash_command, category = "osu!", aliases("c"))]
pub async fn score(
    ctx: Context<'_>,
    #[description = "Beatmap ID to check for a score."] beatmap_url: Option<url::Url>,
    #[description = "Discord user to check score for."] discord_user: Option<
        poise::serenity_prelude::User,
    >,
    #[rest]
    #[description = "osu! user to see score for."]
    user: Option<String>,
) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get().await?;

    let discord_user = discord_user.as_ref().unwrap_or_else(|| ctx.author());

    let Some(osu_user) = get_user(ctx, discord_user, user, connection).await? else { return Ok(()) };

    let beatmap_info: BeatmapInfo;
    if let Some(beatmap_url) = beatmap_url {
        beatmap_info = get_beatmap_info(beatmap_url.as_str())?;
        let Some(_) = beatmap_info.beatmap_id else {
            ctx.say("Please link to a specific beatmap difficulty.")
                .await?;
            return Ok(());
        };
    } else if let Some(found_info) = find_beatmap_link(ctx).await? {
        beatmap_info = found_info;
    } else {
        ctx.say("No beatmap link found.").await?;
        return Ok(());
    }

    let mode = if let Some(mode) = beatmap_info.mode {
        mode
    } else {
        osu_user.mode
    };

    let score = ctx
        .data()
        .osu_client
        .beatmap_user_score(
            beatmap_info
                .beatmap_id
                .ok_or("Failed to get beatmap ID in score command")? as u32,
            osu_user.user_id,
        )
        .mode(mode)
        .await;

    match score {
        Ok(score) => {
            let beatmap = crate::utils::osu::caching::get_beatmap(
                connection,
                ctx.data().osu_client.clone(),
                score.score.map_id,
            )
            .await?;

            let beatmapset = crate::utils::osu::caching::get_beatmapset(
                connection,
                ctx.data().osu_client.clone(),
                beatmap.beatmapset_id as u32,
            )
            .await?;

            send_score_embed(
                ctx,
                ctx.author(),
                &score.score,
                &beatmap,
                &beatmapset,
                osu_user,
                Some(&score.pos),
            )
            .await?;
        }
        Err(why) => {
            ctx.say(format!("Failed to get beatmap score. {why}"))
                .await?;
        }
    }

    Ok(())
}

/// Display a list of your scores on a beatmap.
#[poise::command(prefix_command, slash_command, category = "osu!")]
pub async fn scores(
    ctx: Context<'_>,
    #[description = "Beatmap ID to check for scores."] beatmap_url: Option<url::Url>,
    #[description = "Sort your scores by something other than pp."] sort_type: Option<SortChoices>,
    #[description = "Discord user to check score for."] discord_user: Option<
        poise::serenity_prelude::User,
    >,
    #[rest]
    #[description = "User to see scores for."]
    user: Option<String>,
) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get().await?;

    let discord_user = discord_user.as_ref().unwrap_or_else(|| ctx.author());

    let Some(osu_user) = get_user(ctx, discord_user, user, connection).await? else { return Ok(()) };

    let beatmap_info: BeatmapInfo;
    if let Some(beatmap_url) = beatmap_url {
        beatmap_info = get_beatmap_info(beatmap_url.as_str())?;
        let Some(_) = beatmap_info.beatmap_id else {
            ctx.say("Please link to a specific beatmap difficulty.")
                .await?;
            return Ok(());
        };
    } else if let Some(found_info) = find_beatmap_link(ctx).await? {
        beatmap_info = found_info;
    } else {
        ctx.say("No beatmap link found.").await?;
        return Ok(());
    }

    let beatmap_id = beatmap_info
        .beatmap_id
        .ok_or("Failed to get beatmap ID in scores command")?;

    let api_scores = ctx
        .data()
        .osu_client
        .beatmap_user_scores(beatmap_id as u32, osu_user.user_id)
        .mode(osu_user.mode)
        .await;

    match api_scores {
        Ok(api_scores) => {
            if api_scores.is_empty() {
                ctx.say(format!(
                    "No scores found for {} found on selected beatmap.",
                    osu_user.username
                ))
                .await?;
                return Ok(());
            }

            let mut beatmap_scores: Vec<(Score, usize)> = Vec::new();

            for (pos, score) in api_scores.iter().enumerate() {
                beatmap_scores.push((score.clone(), pos + 1));
            }

            if let Some(sort_type) = sort_type {
                beatmap_scores = sort_scores(beatmap_scores, &sort_type);
            }

            let beatmap = crate::utils::osu::caching::get_beatmap(
                connection,
                ctx.data().osu_client.clone(),
                beatmap_id as u32,
            )
            .await?;

            let beatmapset = crate::utils::osu::caching::get_beatmapset(
                connection,
                ctx.data().osu_client.clone(),
                beatmap.beatmapset_id as u32,
            )
            .await?;

            send_scores_embed(
                ctx,
                ctx.author(),
                connection,
                &beatmap_scores,
                &osu_user,
                beatmap_scores.len() > 5,
                &beatmapset.list_cover,
                Some(&beatmap),
                Some(&beatmapset),
            )
            .await?;
        }
        Err(why) => {
            ctx.say(format!("Failed to get beatmap scores. {why}"))
                .await?;
        }
    }

    Ok(())
}

/// Display your most recent osu score.
#[poise::command(
    prefix_command,
    slash_command,
    category = "osu!",
    aliases("last", "new", "r")
)]
pub async fn recent(
    ctx: Context<'_>,
    #[description = "User to see profile for."] mode: Option<GameModeChoices>,
    #[description = "Discord user to check score for."] discord_user: Option<
        poise::serenity_prelude::User,
    >,
    #[rest]
    #[description = "User to see profile for."]
    user: Option<String>,
) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get().await?;

    let discord_user = discord_user.as_ref().unwrap_or_else(|| ctx.author());

    let Some(osu_user) = get_user(ctx, discord_user, user, connection).await? else { return Ok(()) };

    let mode = if let Some(mode) = mode {
        match mode {
            GameModeChoices::Standard => GameMode::Osu,
            GameModeChoices::Taiko => GameMode::Taiko,
            GameModeChoices::Catch => GameMode::Catch,
            GameModeChoices::Mania => GameMode::Mania,
        }
    } else {
        osu_user.mode
    };

    let recent_score = ctx
        .data()
        .osu_client
        .user_scores(osu_user.user_id)
        .recent()
        .mode(mode)
        .include_fails(true)
        .limit(1)
        .await;

    match recent_score {
        Ok(scores) => {
            if scores.is_empty() {
                ctx.say(format!("No recent scores found for {}.", osu_user.username))
                    .await?;
            } else {
                let score = &scores[0];

                let beatmap = crate::utils::osu::caching::get_beatmap(
                    connection,
                    ctx.data().osu_client.clone(),
                    score.map_id,
                )
                .await?;

                let beatmapset = crate::utils::osu::caching::get_beatmapset(
                    connection,
                    ctx.data().osu_client.clone(),
                    beatmap.beatmapset_id as u32,
                )
                .await?;

                send_score_embed(
                    ctx,
                    ctx.author(),
                    score,
                    &beatmap,
                    &beatmapset,
                    osu_user,
                    None,
                )
                .await?;
            }
        }
        Err(why) => {
            ctx.say(format!("Failed to get recent scores. {why}"))
                .await?;
        }
    }

    Ok(())
}

/// Display your most recent osu score.
#[poise::command(prefix_command, slash_command, category = "osu!", aliases("rb"))]
pub async fn recent_best(
    ctx: Context<'_>,
    #[description = "User to see profile for."] mode: Option<GameModeChoices>,
    #[description = "Discord user to check score for."] discord_user: Option<
        poise::serenity_prelude::User,
    >,
    #[rest]
    #[description = "User to see profile for."]
    user: Option<String>,
) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get().await?;

    let discord_user = discord_user.as_ref().unwrap_or_else(|| ctx.author());

    let Some(osu_user) = get_user(ctx, discord_user, user, connection).await? else { return Ok(()) };

    let mode = if let Some(mode) = mode {
        match mode {
            GameModeChoices::Standard => GameMode::Osu,
            GameModeChoices::Taiko => GameMode::Taiko,
            GameModeChoices::Catch => GameMode::Catch,
            GameModeChoices::Mania => GameMode::Mania,
        }
    } else {
        osu_user.mode
    };

    let recent_score = ctx
        .data()
        .osu_client
        .user_scores(osu_user.user_id)
        .recent()
        .mode(mode)
        .include_fails(false)
        .limit(100)
        .await;

    match recent_score {
        Ok(mut api_scores) => {
            if api_scores.is_empty() {
                ctx.say(format!("No recent scores found for {}.", osu_user.username))
                    .await?;
            } else {
                api_scores.sort_by(|a, b| b.pp.unwrap_or(0.0).total_cmp(&a.pp.unwrap_or(0.0)));
                let score = &api_scores[0];

                let beatmap = crate::utils::osu::caching::get_beatmap(
                    connection,
                    ctx.data().osu_client.clone(),
                    score.map_id,
                )
                .await?;

                let beatmapset = crate::utils::osu::caching::get_beatmapset(
                    connection,
                    ctx.data().osu_client.clone(),
                    beatmap.beatmapset_id as u32,
                )
                .await?;

                send_score_embed(
                    ctx,
                    ctx.author(),
                    score,
                    &beatmap,
                    &beatmapset,
                    osu_user,
                    None,
                )
                .await?;
            }
        }
        Err(why) => {
            ctx.say(format!("Failed to get recent scores. {why}"))
                .await?;
        }
    }

    Ok(())
}

#[derive(poise::ChoiceParameter)]
pub enum SortChoices {
    #[name = "Recent"]
    #[name = "Newest"]
    Recent,
    Oldest,
    #[name = "Accuracy"]
    #[name = "Acc"]
    Accuracy,
    Combo,
    Score,
    PP,
}

/// Display a list of your pinned scores.
#[poise::command(prefix_command, slash_command, category = "osu!")]
pub async fn pins(
    ctx: Context<'_>,
    #[description = "Sort your pins by something else."] sort_type: Option<SortChoices>,
    #[description = "Discord user to check score for."] discord_user: Option<
        poise::serenity_prelude::User,
    >,
    #[rest]
    #[description = "User to see pins for."]
    user: Option<String>,
) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get().await?;

    let discord_user = discord_user.as_ref().unwrap_or_else(|| ctx.author());

    let Some(osu_user) = get_user(ctx, discord_user, user, connection).await? else { return Ok(()) };

    let pinned_scores = ctx
        .data()
        .osu_client
        .user_scores(osu_user.user_id)
        .pinned()
        .mode(osu_user.mode)
        .limit(100)
        .await;

    match pinned_scores {
        Ok(api_scores) => {
            if api_scores.is_empty() {
                ctx.say(format!("No pinned scores found for {}.", osu_user.username))
                    .await?;
                return Ok(());
            }

            let mut pinned_scores: Vec<(Score, usize)> = Vec::new();
            for (pos, score) in api_scores.iter().enumerate() {
                pinned_scores.push((score.clone(), pos + 1));
            }

            if let Some(sort_type) = sort_type {
                pinned_scores = sort_scores(pinned_scores, &sort_type);
            }

            send_scores_embed(
                ctx,
                ctx.author(),
                connection,
                &pinned_scores,
                &osu_user,
                pinned_scores.len() > 5,
                &osu_user.avatar_url,
                None,
                None,
            )
            .await?;
        }
        Err(why) => {
            ctx.say(format!("Failed to get pinned scores. {why}"))
                .await?;
        }
    }

    Ok(())
}

/// Display a list of your #1 scores.
#[poise::command(prefix_command, slash_command, category = "osu!")]
pub async fn firsts(
    ctx: Context<'_>,
    #[description = "Sort your #1 scores by something else."] sort_type: Option<SortChoices>,
    #[description = "Discord user to check score for."] discord_user: Option<
        poise::serenity_prelude::User,
    >,
    #[rest]
    #[description = "User to see firsts for."]
    user: Option<String>,
) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get().await?;

    let discord_user = discord_user.as_ref().unwrap_or_else(|| ctx.author());

    let Some(osu_user) = get_user(ctx, discord_user, user, connection).await? else { return Ok(()) };

    let first_scores = ctx
        .data()
        .osu_client
        .user_scores(osu_user.user_id)
        .firsts()
        .mode(osu_user.mode)
        .limit(100)
        .await;

    match first_scores {
        Ok(api_scores) => {
            if api_scores.is_empty() {
                ctx.say(format!("No first scores found for {}.", osu_user.username))
                    .await?;
                return Ok(());
            }

            let mut first_scores: Vec<(Score, usize)> = Vec::new();
            for (pos, score) in api_scores.iter().enumerate() {
                first_scores.push((score.clone(), pos + 1));
            }

            if let Some(sort_type) = sort_type {
                first_scores = sort_scores(first_scores, &sort_type);
            }

            send_scores_embed(
                ctx,
                ctx.author(),
                connection,
                &first_scores,
                &osu_user,
                first_scores.len() > 5,
                &osu_user.avatar_url,
                None,
                None,
            )
            .await?;
        }
        Err(why) => {
            ctx.say(format!("Failed to get first scores. {why}"))
                .await?;
        }
    }

    Ok(())
}

/// Display a list of your top scores.
#[poise::command(prefix_command, slash_command, category = "osu!")]
pub async fn top(
    ctx: Context<'_>,
    #[description = "Sort your top scores by something else."] sort_type: Option<SortChoices>,
    #[description = "Discord user to check score for."] discord_user: Option<
        poise::serenity_prelude::User,
    >,
    #[rest]
    #[description = "User to see profile for."]
    user: Option<String>,
) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get().await?;

    let discord_user = discord_user.as_ref().unwrap_or_else(|| ctx.author());

    let Some(osu_user) = get_user(ctx, discord_user, user, connection).await? else { return Ok(()) };

    let best_scores = ctx
        .data()
        .osu_client
        .user_scores(osu_user.user_id)
        .best()
        .mode(osu_user.mode)
        .limit(100)
        .await;
    match best_scores {
        Ok(api_scores) => {
            if api_scores.is_empty() {
                ctx.say(format!("No top scores found for {}.", osu_user.username))
                    .await?;
                return Ok(());
            }

            let mut best_scores: Vec<(Score, usize)> = Vec::new();
            for (pos, score) in api_scores.iter().enumerate() {
                best_scores.push((score.clone(), pos + 1));
            }

            if let Some(sort_type) = sort_type {
                best_scores = sort_scores(best_scores, &sort_type);
            }

            send_scores_embed(
                ctx,
                ctx.author(),
                connection,
                &best_scores,
                &osu_user,
                best_scores.len() > 5,
                &osu_user.avatar_url,
                None,
                None,
            )
            .await?;
        }
        Err(why) => {
            ctx.say(format!("Failed to get best scores. {why}")).await?;
        }
    }

    Ok(())
}

#[poise::command(prefix_command, slash_command, category = "osu!", guild_only)]
pub async fn score_notifications(
    ctx: Context<'_>,
    #[description = "Channel to notify scores in"] scores_channel: GuildChannel,
) -> Result<(), Error> {
    let guild = ctx
        .guild()
        .ok_or("Failed to get guild in score_notifications command")?
        .clone();
    let connection = &mut ctx.data().db_pool.get().await?;
    let new_item = match osu_guild_channels::read(connection, guild.id.0.get() as i64).await {
        Ok(guild_config) => NewOsuGuildChannel {
            guild_id: guild_config.guild_id,
            score_channel: Some(scores_channel.id.0.get() as i64),
            map_channel: guild_config.map_channel,
        },
        Err(_) => NewOsuGuildChannel {
            guild_id: guild.id.0.get() as i64,
            score_channel: Some(scores_channel.id.0.get() as i64),
            map_channel: None,
        },
    };

    osu_guild_channels::create(connection, &new_item).await?;

    ctx.say("Updated your guild's score notification channel!")
        .await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, category = "osu!", guild_only)]
pub async fn map_notifications(
    ctx: Context<'_>,
    #[description = "Channel to notify maps in"] map_channel: GuildChannel,
) -> Result<(), Error> {
    let guild = ctx
        .guild()
        .ok_or("Failed to get guild in map_notifications command")?
        .clone();
    let connection = &mut ctx.data().db_pool.get().await?;
    let new_item = match osu_guild_channels::read(connection, guild.id.0.get() as i64).await {
        Ok(guild_config) => NewOsuGuildChannel {
            guild_id: guild_config.guild_id,
            score_channel: guild_config.score_channel,
            map_channel: Some(map_channel.id.0.get() as i64),
        },
        Err(_) => NewOsuGuildChannel {
            guild_id: guild.id.0.get() as i64,
            score_channel: None,
            map_channel: Some(map_channel.id.0.get() as i64),
        },
    };

    osu_guild_channels::create(connection, &new_item).await?;

    ctx.say("Updated your guild's map notification channel!")
        .await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, category = "osu!", guild_only)]
pub async fn delete_guild_config(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx
        .guild()
        .ok_or("Failed to get guild in delete_guild_config command")?
        .clone();
    let connection = &mut ctx.data().db_pool.get().await?;
    match osu_guild_channels::read(connection, guild.id.0.get() as i64).await {
        Ok(guild_config) => {
            osu_guild_channels::delete(connection, guild_config.guild_id).await?;
            ctx.say("Your guild's config has been deleted.").await?;
        }
        Err(_) => {
            ctx.say("Your guild doesn't have a config stored.").await?;
        }
    };

    Ok(())
}

#[poise::command(prefix_command, category = "osu!", guild_only, owners_only)]
pub async fn debug(ctx: Context<'_>) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get().await?;
    let linked_profiles = linked_osu_profiles::get_all(connection).await?;
    let tracked_profiles = osu_users::get_all(connection).await?;

    let mut playing_users: Vec<String> = Vec::new();
    for linked_profile in &linked_profiles {
        for osu_user in &tracked_profiles {
            if linked_profile.osu_id == osu_user.id {
                let user = ctx
                    .cache()
                    .ok_or("Failed to retrieve discord cache in debug command")?
                    .user(linked_profile.id as u64);
                if let Some(user) = user {
                    if is_playing(ctx.discord(), user.id, linked_profile.home_guild)? {
                        playing_users.push(format!("`{}`", user.name.clone()));
                    }
                } else {
                    continue;
                }
            }
        }
    }

    let formatted_playing_members = if playing_users.is_empty() {
        "None".into()
    } else {
        playing_users.join(", ")
    };

    let formatted_message = format!(
        "Members registered as playing: {}\n\
         Total members tracked: `{}`",
        formatted_playing_members,
        tracked_profiles.len()
    );

    ctx.say(formatted_message).await?;

    Ok(())
}
