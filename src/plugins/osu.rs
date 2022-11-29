use crate::models::linked_osu_profiles::NewLinkedOsuProfile;
use crate::utils::db::{linked_osu_profiles, osu_guild_channels, osu_notifications};
use crate::utils::osu::misc::{gamemode_from_string, get_user, sort_scores, wipe_profile_data};
use crate::utils::osu::misc_format::format_missing_user_string;
use chrono::Utc;
use poise::{serenity_prelude, CreateReply};
use rosu_v2::prelude::Score;
use serenity_prelude::model::colour::colours::roles::BLUE;
use serenity_prelude::{Colour, CreateEmbed, CreateEmbedAuthor, GuildChannel};

use crate::models::osu_guild_channels::NewOsuGuildChannel;
use crate::models::osu_notifications::NewOsuNotification;
use crate::{Context, Error};

use crate::utils::osu::embeds::{send_score_embed, send_scores_embed};
use crate::utils::osu::regex::get_beatmap_info;

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
        "pins",
        "firsts",
        "top",
        "score_notifications",
        "map_notifications",
        "delete_guild_config"
    )
)]
pub async fn osu(ctx: Context<'_>) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get()?;
    let profile = linked_osu_profiles::read(connection, ctx.author().id.0.get() as i64);
    match profile {
        Ok(profile) => {
            let color: Colour;
            if let Some(guild) = ctx.guild() {
                if let Some(member) = ctx.cache_and_http().cache.member(guild.id, ctx.author().id) {
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
            ctx.say(format_missing_user_string(ctx, ctx.author()).await)
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
    let connection = &mut ctx.data().db_pool.get()?;

    if let Ok(profile) = linked_osu_profiles::read(connection, ctx.author().id.0.get() as i64) {
        linked_osu_profiles::delete(connection, profile.id)?;
        wipe_profile_data(connection, profile.osu_id)?;
    }

    let query_item = NewLinkedOsuProfile {
        id: ctx.author().id.0.get() as i64,
        osu_id: i64::from(user.user_id),
        home_guild: ctx.guild_id().unwrap().0.get() as i64,
        mode: user.mode.to_string(),
    };

    let notification_item = NewOsuNotification {
        id: i64::from(user.user_id),
        last_pp: Utc::now(),
        last_event: Utc::now(),
    };
    osu_notifications::create(connection, &notification_item)?;

    linked_osu_profiles::create(connection, &query_item)?;

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
    let connection = &mut ctx.data().db_pool.get()?;
    let profile = linked_osu_profiles::read(connection, ctx.author().id.0.get() as i64);

    match profile {
        Ok(profile) => {
            linked_osu_profiles::delete(connection, profile.id)?;
            wipe_profile_data(connection, profile.osu_id)?;
            ctx.say("Unlinked your profile.").await?;
        }
        Err(_) => {
            ctx.say(format_missing_user_string(ctx, ctx.author()).await)
                .await?;
        }
    };

    Ok(())
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
    #[description = "Gamemode to switch to."] mode: String,
) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get()?;
    let profile = linked_osu_profiles::read(connection, ctx.author().id.0.get() as i64);
    match profile {
        Ok(profile) => {
            let parsed_mode = if let Some(mode) = gamemode_from_string(&mode) {
                mode
            } else {
                ctx.say("Invalid gamemode specified.").await?;
                return Ok(());
            };

            let query_item = NewLinkedOsuProfile {
                id: profile.id,
                osu_id: profile.osu_id,
                home_guild: profile.home_guild,
                mode: parsed_mode.to_string(),
            };

            linked_osu_profiles::update(connection, profile.id, &query_item)?;
            wipe_profile_data(connection, profile.osu_id)?;

            ctx.say(format!("Updated your osu! mode to {}.", parsed_mode))
                .await?;
        }
        Err(_) => {
            ctx.say(format_missing_user_string(ctx, ctx.author()).await)
                .await?;
        }
    }

    Ok(())
}

/// Display your score on a beatmap.
#[poise::command(prefix_command, slash_command, category = "osu!", aliases("c"))]
pub async fn score(
    ctx: Context<'_>,
    #[description = "Beatmap ID to check for a score."] beatmap_url: String,
    #[rest]
    #[description = "User to see score for."]
    user: Option<String>,
) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get()?;

    let osu_user = match get_user(ctx, user, connection).await? {
        Some(user) => user,
        _ => return Ok(()),
    };

    let beatmap_info = get_beatmap_info(&beatmap_url);
    let beatmap_id = if let Some(id) = beatmap_info.beatmap_id {
        id
    } else {
        ctx.say("Please link to a specific beatmap difficulty.")
            .await?;
        return Ok(());
    };

    let mode = if let Some(mode) = beatmap_info.mode {
        mode
    } else {
        osu_user.mode
    };

    let score = ctx
        .data()
        .osu_client
        .beatmap_user_score(beatmap_id as u32, osu_user.user_id)
        .mode(mode)
        .await;

    match score {
        Ok(score) => {
            let beatmap = crate::utils::osu::caching::get_beatmap(
                connection,
                ctx.data().osu_client.clone(),
                score.score.map.as_ref().unwrap().map_id,
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
            ctx.say(format!("Failed to get beatmap score. {}", why))
                .await?;
        }
    }

    Ok(())
}

/// Display a list of your scores on a beatmap.
#[poise::command(prefix_command, slash_command, category = "osu!")]
pub async fn scores(
    ctx: Context<'_>,
    #[description = "Beatmap ID to check for scores."] beatmap_url: String,
    #[description = "Sort your scores by something other than pp."] sort_type: Option<SortChoices>,
    #[rest]
    #[description = "User to see scores for."]
    user: Option<String>,
) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get()?;

    let osu_user = match get_user(ctx, user, connection).await? {
        Some(user) => user,
        _ => return Ok(()),
    };

    let beatmap_info = get_beatmap_info(&beatmap_url);
    let beatmap_id = if let Some(id) = beatmap_info.beatmap_id {
        id
    } else {
        ctx.say("Please link to a specific beatmap difficulty.")
            .await?;
        return Ok(());
    };

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
            ctx.say(format!("Failed to get beatmap scores. {}", why))
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
    #[rest]
    #[description = "User to see profile for."]
    user: Option<String>,
) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get()?;

    let osu_user = match get_user(ctx, user, connection).await? {
        Some(user) => user,
        _ => return Ok(()),
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

                let beatmap = crate::utils::osu::caching::get_beatmap(
                    connection,
                    ctx.data().osu_client.clone(),
                    score.map.as_ref().unwrap().map_id,
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
            ctx.say(format!("Failed to get recent scores. {}", why))
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
    #[rest]
    #[description = "User to see pins for."]
    user: Option<String>,
) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get()?;

    let osu_user = match get_user(ctx, user, connection).await? {
        Some(user) => user,
        _ => return Ok(()),
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
            ctx.say(format!("Failed to get pinned scores. {}", why))
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
    #[rest]
    #[description = "User to see firsts for."]
    user: Option<String>,
) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get()?;

    let osu_user = match get_user(ctx, user, connection).await? {
        Some(user) => user,
        _ => return Ok(()),
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
            ctx.say(format!("Failed to get first scores. {}", why))
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
    #[rest]
    #[description = "User to see profile for."]
    user: Option<String>,
) -> Result<(), Error> {
    let connection = &mut ctx.data().db_pool.get()?;

    let osu_user = match get_user(ctx, user, connection).await? {
        Some(user) => user,
        _ => return Ok(()),
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
            ctx.say(format!("Failed to get best scores. {}", why))
                .await?;
        }
    }

    Ok(())
}

#[poise::command(prefix_command, slash_command, category = "osu!", guild_only)]
pub async fn score_notifications(
    ctx: Context<'_>,
    #[description = "Channel to notify scores in"] scores_channel: GuildChannel,
) -> Result<(), Error> {
    let guild = ctx.guild().unwrap().clone();
    let connection = &mut ctx.data().db_pool.get()?;
    let new_item = match osu_guild_channels::read(connection, guild.id.0.get() as i64) {
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

    osu_guild_channels::create(connection, &new_item)?;

    ctx.say("Updated your guild's score notification channel!")
        .await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, category = "osu!", guild_only)]
pub async fn map_notifications(
    ctx: Context<'_>,
    #[description = "Channel to notify scores in"] map_channel: GuildChannel,
) -> Result<(), Error> {
    let guild = ctx.guild().unwrap().clone();
    let connection = &mut ctx.data().db_pool.get()?;
    let new_item = match osu_guild_channels::read(connection, guild.id.0.get() as i64) {
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

    osu_guild_channels::create(connection, &new_item)?;

    ctx.say("Updated your guild's map notification channel!")
        .await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, category = "osu!", guild_only)]
pub async fn delete_guild_config(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx.guild().unwrap().clone();
    let connection = &mut ctx.data().db_pool.get()?;
    match osu_guild_channels::read(connection, guild.id.0.get() as i64) {
        Ok(guild_config) => {
            osu_guild_channels::delete(connection, guild_config.guild_id)?;
            ctx.say("Your guild's config has been deleted.").await?;
        }
        Err(_) => {
            ctx.say("Your guild doesn't have a config stored.").await?;
        }
    };

    Ok(())
}
