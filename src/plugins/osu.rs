use crate::models::linked_osu_profiles::NewLinkedOsuProfile;
use crate::models::osu_guild_channels::NewOsuGuildChannel;
use crate::models::osu_notifications::NewOsuNotification;
use crate::utils::db::{
    beatmaps, beatmapsets, linked_osu_profiles, osu_file, osu_guild_channels, osu_notifications,
    osu_users,
};
use crate::utils::misc::get_reply;
use crate::utils::osu::caching::{get_beatmap, get_beatmapset};
use crate::utils::osu::calculate::calculate;
use crate::utils::osu::card::render_card;
use crate::utils::osu::embeds::{send_score_embed, send_scores_embed};
use crate::utils::osu::map_format::format_map_status;
use crate::utils::osu::misc::{
    calculate_potential_acc, find_beatmap_link, get_osu_user, get_user, is_playing,
    set_up_score_list, sort_scores, wipe_profile_data,
};
use crate::utils::osu::misc_format::format_missing_user_string;
use crate::utils::osu::regex::{BeatmapInfo, get_beatmap_info};
use crate::{Context, Error};
use chrono::Utc;
use poise::CreateReply;
use poise::serenity_prelude::model::colour::colours::roles::BLUE;
use poise::serenity_prelude::{
    CreateAttachment, CreateEmbed, CreateEmbedAuthor, GetMessages, GuildChannel, UserId,
};
use rosu_v2::model::GameMode;

/// Display information about your osu! user.
#[poise::command(
    prefix_command,
    slash_command,
    category = "osu!",
    subcommands(
        "link",
        "mapinfo",
        "score",
        "scores",
        "unlink",
        "mode",
        "recent",
        "recent_best",
        "recent_list",
        "pins",
        "firsts",
        "top",
        "score_notifications",
        "map_notifications",
        "delete_guild_config",
        "debug"
    )
)]
pub async fn osu(
    ctx: Context<'_>,
    #[description = "Mode to see profile in."] mode: Option<GameModeChoices>,
    #[description = "Discord user to see profile for."] discord_user: Option<
        poise::serenity_prelude::User,
    >,
    #[rest]
    #[description = "User to see profile for."]
    user: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let connection = &mut ctx.data().db_pool.get().await?;

    let discord_user = discord_user.as_ref().unwrap_or_else(|| ctx.author());

    let Some(osu_user) = get_user(ctx, discord_user, user, connection, mode).await? else {
        return Ok(());
    };

    let color = match ctx.guild() {
        Some(guild) => match guild.members.get(&discord_user.id) {
            Some(member) => member.colour(ctx.cache()).unwrap_or(BLUE),
            _ => BLUE,
        },
        _ => BLUE,
    };

    let author = CreateEmbedAuthor::new(discord_user.name.clone()).icon_url(discord_user.face());

    let card = render_card(&osu_user, color).await?.encode_png()?;

    let embed = CreateEmbed::new()
        .image("attachment://card.png")
        .author(author);

    let file = CreateAttachment::bytes(card, "card.png");

    let builder = CreateReply::default().embed(embed).attachment(file);

    ctx.send(builder).await?;

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
    ctx.defer().await?;
    let user = ctx.data().osu_client.user(username).await?;
    let connection = &mut ctx.data().db_pool.get().await?;

    if let Ok(profile) =
        linked_osu_profiles::read(connection, i64::try_from(ctx.author().id.get())?).await
    {
        linked_osu_profiles::delete(connection, profile.id).await?;
        wipe_profile_data(connection, profile.osu_id).await?;
    }

    let query_item = NewLinkedOsuProfile {
        id: i64::try_from(ctx.author().id.get())?,
        osu_id: i64::from(user.user_id),
        home_guild: i64::try_from(
            ctx.guild_id()
                .ok_or("Failed to get guild ID in link command")?
                .get(),
        )?,
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
    ctx.defer().await?;
    let connection = &mut ctx.data().db_pool.get().await?;
    let profile =
        linked_osu_profiles::read(connection, i64::try_from(ctx.author().id.get())?).await;

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
    }

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

impl From<GameModeChoices> for GameMode {
    fn from(gamemode: GameModeChoices) -> GameMode {
        match gamemode {
            GameModeChoices::Standard => GameMode::Osu,
            GameModeChoices::Taiko => GameMode::Taiko,
            GameModeChoices::Catch => GameMode::Catch,
            GameModeChoices::Mania => GameMode::Mania,
        }
    }
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
    ctx.defer().await?;
    let connection = &mut ctx.data().db_pool.get().await?;
    let profile =
        linked_osu_profiles::read(connection, i64::try_from(ctx.author().id.get())?).await;

    let mode: GameMode = new_mode.into();

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
pub async fn mapinfo(
    ctx: Context<'_>,
    #[description = "Beatmap ID to check for a score."] beatmap_url: Option<url::Url>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let connection = &mut ctx.data().db_pool.get().await?;

    let beatmap_info: BeatmapInfo;
    let reply = get_reply(ctx);

    if let Some(beatmap_url) = beatmap_url {
        beatmap_info = get_beatmap_info(beatmap_url.as_str())?;
        let Some(_) = beatmap_info.beatmapset_id else {
            ctx.say("Please link to a beatmapset.").await?;
            return Ok(());
        };
    } else if let Some(reply) = reply {
        if let Some(found_info) = find_beatmap_link(vec![reply]).await? {
            beatmap_info = found_info;
        } else {
            ctx.say("No beatmap link found.").await?;
            return Ok(());
        }
    } else if let Some(found_info) = find_beatmap_link(
        ctx.channel_id()
            .messages(ctx.http(), GetMessages::new().limit(100))
            .await?,
    )
    .await?
    {
        beatmap_info = found_info;
    } else {
        ctx.say("No beatmap link found.").await?;
        return Ok(());
    }

    let beatmapset = get_beatmapset(
        connection,
        ctx.data().osu_client.clone(),
        u32::try_from(
            beatmap_info
                .beatmapset_id
                .ok_or("Failed to get beatmapset id in mapinfo command")?,
        )?,
    )
    .await?;

    let color = match ctx.author_member().await {
        None => BLUE,
        Some(member) => member.colour(ctx.cache()).unwrap_or(BLUE),
    };

    let embed = format_map_status(beatmapset, color)?;

    let builder = CreateReply::default().embed(embed);

    ctx.send(builder).await?;

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
    ctx.defer().await?;
    let connection = &mut ctx.data().db_pool.get().await?;

    let discord_user = discord_user.as_ref().unwrap_or_else(|| ctx.author());

    let Some(osu_user) = get_user(ctx, discord_user, user, connection, None).await? else {
        return Ok(());
    };

    let beatmap_info: BeatmapInfo;
    let reply = get_reply(ctx);

    if let Some(beatmap_url) = beatmap_url {
        beatmap_info = get_beatmap_info(beatmap_url.as_str())?;
        let Some(_) = beatmap_info.beatmap_id else {
            ctx.say("Please link to a specific beatmap difficulty.")
                .await?;
            return Ok(());
        };
    } else if let Some(reply) = reply {
        if let Some(found_info) = find_beatmap_link(vec![reply]).await? {
            beatmap_info = found_info;
        } else {
            ctx.say("No beatmap link found.").await?;
            return Ok(());
        }
    } else if let Some(found_info) = find_beatmap_link(
        ctx.channel_id()
            .messages(ctx.http(), GetMessages::new().limit(100))
            .await?,
    )
    .await?
    {
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
            u32::try_from(
                beatmap_info
                    .beatmap_id
                    .ok_or("Failed to get beatmap ID in score command")?,
            )?,
            osu_user.user_id,
        )
        .mode(mode)
        .await;

    match score {
        Ok(score) => {
            let beatmap = get_beatmap(
                connection,
                ctx.data().osu_client.clone(),
                score.score.map_id,
            )
            .await?;

            let calculated_results = calculate(
                Some(&score.score),
                &beatmap.0,
                &beatmap.2,
                calculate_potential_acc(&score.score),
            )?;

            send_score_embed(
                ctx,
                (&score.score, &beatmap.0, &beatmap.1, &calculated_results),
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
    ctx.defer().await?;
    let connection = &mut ctx.data().db_pool.get().await?;

    let discord_user = discord_user.as_ref().unwrap_or_else(|| ctx.author());

    let Some(osu_user) = get_user(ctx, discord_user, user, connection, None).await? else {
        return Ok(());
    };

    let beatmap_info: BeatmapInfo;
    let reply = get_reply(ctx);

    if let Some(beatmap_url) = beatmap_url {
        beatmap_info = get_beatmap_info(beatmap_url.as_str())?;
        let Some(_) = beatmap_info.beatmap_id else {
            ctx.say("Please link to a specific beatmap difficulty.")
                .await?;
            return Ok(());
        };
    } else if let Some(reply) = reply {
        if let Some(found_info) = find_beatmap_link(vec![reply]).await? {
            beatmap_info = found_info;
        } else {
            ctx.say("No beatmap link found.").await?;
            return Ok(());
        }
    } else if let Some(found_info) = find_beatmap_link(
        ctx.channel_id()
            .messages(ctx.http(), GetMessages::new().limit(100))
            .await?,
    )
    .await?
    {
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
        .beatmap_user_scores(u32::try_from(beatmap_id)?, osu_user.user_id)
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

            let mut beatmap_scores = set_up_score_list(&ctx, connection, api_scores).await?;

            if let Some(sort_type) = sort_type {
                beatmap_scores = sort_scores(beatmap_scores, &sort_type);
            }

            let beatmap = get_beatmap(
                connection,
                ctx.data().osu_client.clone(),
                u32::try_from(beatmap_id)?,
            )
            .await?;

            send_scores_embed(ctx, beatmap_scores, &osu_user, &beatmap.1.list_cover).await?;
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
    #[description = "Gamemode to look for scores."] mode: Option<GameModeChoices>,
    #[description = "Discord user to check score for."] discord_user: Option<
        poise::serenity_prelude::User,
    >,
    #[rest]
    #[description = "User to see score for."]
    user: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let connection = &mut ctx.data().db_pool.get().await?;

    let discord_user = discord_user.as_ref().unwrap_or_else(|| ctx.author());

    let Some(osu_user) = get_user(ctx, discord_user, user, connection, mode).await? else {
        return Ok(());
    };

    let recent_score = ctx
        .data()
        .osu_client
        .user_scores(osu_user.user_id)
        .recent()
        .mode(osu_user.mode)
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

                let beatmap =
                    get_beatmap(connection, ctx.data().osu_client.clone(), score.map_id).await?;

                let calculated_results = calculate(
                    Some(score),
                    &beatmap.0,
                    &beatmap.2,
                    calculate_potential_acc(score),
                )?;

                send_score_embed(
                    ctx,
                    (score, &beatmap.0, &beatmap.1, &calculated_results),
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
    #[description = "Mode to get scores for."] mode: Option<GameModeChoices>,
    #[description = "Discord user to check score for."] discord_user: Option<
        poise::serenity_prelude::User,
    >,
    #[rest]
    #[description = "User to see profile for."]
    user: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let connection = &mut ctx.data().db_pool.get().await?;

    let discord_user = discord_user.as_ref().unwrap_or_else(|| ctx.author());

    let Some(osu_user) = get_user(ctx, discord_user, user, connection, mode).await? else {
        return Ok(());
    };
    let recent_score = ctx
        .data()
        .osu_client
        .user_scores(osu_user.user_id)
        .recent()
        .mode(osu_user.mode)
        .include_fails(false)
        .limit(100)
        .await;

    match recent_score {
        Ok(api_scores) => {
            if api_scores.is_empty() {
                ctx.say(format!("No recent scores found for {}.", osu_user.username))
                    .await?;
            } else {
                let mut recent_scores = set_up_score_list(&ctx, connection, api_scores).await?;

                recent_scores = sort_scores(recent_scores, &SortChoices::PP);
                let score = &recent_scores[0];

                send_score_embed(
                    ctx,
                    (&score.0, &score.2, &score.3, &score.4),
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

/// Display a list of your recent scores.
#[poise::command(prefix_command, slash_command, category = "osu!", aliases("rl"))]
pub async fn recent_list(
    ctx: Context<'_>,
    #[description = "Sort your recent scores by something else."] sort_type: Option<SortChoices>,
    #[description = "Mode to get scores for."] mode: Option<GameModeChoices>,
    #[description = "Discord user to see plays for."] discord_user: Option<
        poise::serenity_prelude::User,
    >,
    #[rest]
    #[description = "User to see plays for."]
    user: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let connection = &mut ctx.data().db_pool.get().await?;

    let discord_user = discord_user.as_ref().unwrap_or_else(|| ctx.author());

    let Some(osu_user) = get_user(ctx, discord_user, user, connection, mode).await? else {
        return Ok(());
    };

    let recent_scores = ctx
        .data()
        .osu_client
        .user_scores(osu_user.user_id)
        .recent()
        .include_fails(false)
        .mode(osu_user.mode)
        .limit(100)
        .await;
    match recent_scores {
        Ok(api_scores) => {
            if api_scores.is_empty() {
                ctx.say(format!("No recent scores found for {}.", osu_user.username))
                    .await?;
                return Ok(());
            }

            let mut best_scores = set_up_score_list(&ctx, connection, api_scores).await?;

            if let Some(sort_type) = sort_type {
                best_scores = sort_scores(best_scores, &sort_type);
            }

            send_scores_embed(ctx, best_scores, &osu_user, &osu_user.avatar_url).await?;
        }
        Err(why) => {
            ctx.say(format!("Failed to get best scores. {why}")).await?;
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
    #[name = "BPM"]
    Bpm,
    #[name = "Stars"]
    #[name = "SR"]
    Stars,
    Length,
    Misses,
}

/// Display a list of your pinned scores.
#[poise::command(prefix_command, slash_command, category = "osu!")]
pub async fn pins(
    ctx: Context<'_>,
    #[description = "Sort your pins by something else."] sort_type: Option<SortChoices>,
    #[description = "Mode to get scores for."] mode: Option<GameModeChoices>,
    #[description = "Discord user to check score for."] discord_user: Option<
        poise::serenity_prelude::User,
    >,
    #[rest]
    #[description = "User to see pins for."]
    user: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let connection = &mut ctx.data().db_pool.get().await?;

    let discord_user = discord_user.as_ref().unwrap_or_else(|| ctx.author());

    let Some(osu_user) = get_user(ctx, discord_user, user, connection, mode).await? else {
        return Ok(());
    };

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

            let mut pinned_scores = set_up_score_list(&ctx, connection, api_scores).await?;

            if let Some(sort_type) = sort_type {
                pinned_scores = sort_scores(pinned_scores, &sort_type);
            }

            send_scores_embed(ctx, pinned_scores, &osu_user, &osu_user.avatar_url).await?;
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
    #[description = "Mode to get scores for."] mode: Option<GameModeChoices>,
    #[description = "Discord user to check score for."] discord_user: Option<
        poise::serenity_prelude::User,
    >,
    #[rest]
    #[description = "User to see firsts for."]
    user: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let connection = &mut ctx.data().db_pool.get().await?;

    let discord_user = discord_user.as_ref().unwrap_or_else(|| ctx.author());

    let Some(osu_user) = get_user(ctx, discord_user, user, connection, mode).await? else {
        return Ok(());
    };

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

            let mut first_scores = set_up_score_list(&ctx, connection, api_scores).await?;

            if let Some(sort_type) = sort_type {
                first_scores = sort_scores(first_scores, &sort_type);
            }

            send_scores_embed(ctx, first_scores, &osu_user, &osu_user.avatar_url).await?;
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
    #[description = "Mode to get scores for."] mode: Option<GameModeChoices>,
    #[description = "Discord user to check score for."] discord_user: Option<
        poise::serenity_prelude::User,
    >,
    #[rest]
    #[description = "User to see profile for."]
    user: Option<String>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let connection = &mut ctx.data().db_pool.get().await?;

    let discord_user = discord_user.as_ref().unwrap_or_else(|| ctx.author());

    let Some(osu_user) = get_user(ctx, discord_user, user, connection, mode).await? else {
        return Ok(());
    };

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

            let mut best_scores = set_up_score_list(&ctx, connection, api_scores).await?;

            if let Some(sort_type) = sort_type {
                best_scores = sort_scores(best_scores, &sort_type);
            }

            send_scores_embed(ctx, best_scores, &osu_user, &osu_user.avatar_url).await?;
        }
        Err(why) => {
            ctx.say(format!("Failed to get best scores. {why}")).await?;
        }
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    category = "osu!",
    guild_only,
    required_permissions = "MANAGE_GUILD"
)]
pub async fn score_notifications(
    ctx: Context<'_>,
    #[description = "Channel to notify scores in"] score_channels: Vec<GuildChannel>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let guild = ctx
        .guild()
        .ok_or("Failed to get guild in score_notifications command")?
        .clone();

    let mut new_score_channels = Vec::new();
    for score_channel in score_channels {
        new_score_channels.push(Some(i64::try_from(score_channel.id.get())?));
    }

    let connection = &mut ctx.data().db_pool.get().await?;
    let new_item = match osu_guild_channels::read(connection, i64::try_from(guild.id.get())?).await
    {
        Ok(guild_config) => NewOsuGuildChannel {
            guild_id: guild_config.guild_id,
            score_channel: Some(new_score_channels),
            map_channel: guild_config.map_channel,
        },
        Err(_) => NewOsuGuildChannel {
            guild_id: i64::try_from(guild.id.get())?,
            score_channel: Some(new_score_channels),
            map_channel: None,
        },
    };

    osu_guild_channels::create(connection, &new_item).await?;

    ctx.say("Updated your guild's score notification channel!")
        .await?;

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    category = "osu!",
    guild_only,
    required_permissions = "MANAGE_GUILD"
)]
pub async fn map_notifications(
    ctx: Context<'_>,
    #[description = "Channel to notify maps in"] map_channels: Vec<GuildChannel>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let guild = ctx
        .guild()
        .ok_or("Failed to get guild in map_notifications command")?
        .clone();

    let mut new_map_channels = Vec::new();
    for map_channel in map_channels {
        new_map_channels.push(Some(i64::try_from(map_channel.id.get())?));
    }
    let connection = &mut ctx.data().db_pool.get().await?;
    let new_item = match osu_guild_channels::read(connection, i64::try_from(guild.id.get())?).await
    {
        Ok(guild_config) => NewOsuGuildChannel {
            guild_id: guild_config.guild_id,
            score_channel: guild_config.score_channel,
            map_channel: Some(new_map_channels),
        },
        Err(_) => NewOsuGuildChannel {
            guild_id: i64::try_from(guild.id.get())?,
            score_channel: None,
            map_channel: Some(new_map_channels),
        },
    };

    osu_guild_channels::create(connection, &new_item).await?;

    ctx.say("Updated your guild's map notification channel!")
        .await?;

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    category = "osu!",
    guild_only,
    required_permissions = "MANAGE_GUILD"
)]
pub async fn delete_guild_config(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let guild = ctx
        .guild()
        .ok_or("Failed to get guild in delete_guild_config command")?
        .clone();
    let connection = &mut ctx.data().db_pool.get().await?;
    match osu_guild_channels::read(connection, i64::try_from(guild.id.get())?).await {
        Ok(guild_config) => {
            osu_guild_channels::delete(connection, guild_config.guild_id).await?;
            ctx.say("Your guild's config has been deleted.").await?;
        }
        Err(_) => {
            ctx.say("Your guild doesn't have a config stored.").await?;
        }
    }

    Ok(())
}

#[poise::command(prefix_command, category = "osu!", owners_only)]
pub async fn debug(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let connection = &mut ctx.data().db_pool.get().await?;
    let linked_profiles = linked_osu_profiles::get_all(connection).await?;
    let tracked_profiles = osu_users::get_all(connection).await?;
    let beatmaps_count = beatmaps::count_entries(connection).await?;
    let beatmapsets_count = beatmapsets::count_entries(connection).await?;
    let guild_channels_count = osu_guild_channels::count_entries(connection).await?;
    let osu_file_count = osu_file::count_entries(connection).await?;

    let mut playing_users: Vec<String> = Vec::new();
    for linked_profile in &linked_profiles {
        if tracked_profiles
            .iter()
            .any(|x| x.id == linked_profile.osu_id)
        {
            let user = get_osu_user(
                &ctx.serenity_context().cache,
                UserId::from(u64::try_from(linked_profile.id)?),
                u64::try_from(linked_profile.home_guild)?,
            )?;
            if let Some(user) = user {
                if is_playing(
                    &ctx.serenity_context().cache,
                    user.id,
                    linked_profile.home_guild,
                )? {
                    playing_users.push(format!("`{}`", user.name));
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
         Total members tracked: `{}`\n\
         Total linked profiles: `{}`\n\
         Total beatmaps cached: `{}`\n\
         Total beatmapsets cached: `{}`\n\
         Total osu files cached: `{}`\n\
         Total guilds with configs: `{}`",
        formatted_playing_members,
        tracked_profiles.len(),
        linked_profiles.len(),
        beatmaps_count,
        beatmapsets_count,
        osu_file_count,
        guild_channels_count
    );

    ctx.say(formatted_message).await?;

    Ok(())
}
