use crate::models::linked_osu_profiles::NewLinkedOsuProfile;
use crate::utils::db::{linked_osu_profiles, osu_guild_channels, osu_notifications, osu_users};
use crate::utils::osu::misc::{gamemode_from_string, wipe_profile_data};
use crate::utils::osu::misc_format::format_missing_user_string;
use chrono::Utc;
use poise::{serenity_prelude, CreateReply};
use rosu_v2::prelude::Score;
use serenity_prelude::model::colour::colours::roles::BLUE;
use serenity_prelude::{Colour, CreateEmbed, CreateEmbedAuthor, GuildChannel, User};

use crate::models::osu_guild_channels::NewOsuGuildChannel;
use crate::models::osu_notifications::NewOsuNotification;
use crate::{Context, Error};

use crate::utils::osu::embeds::{send_score_embed, send_top_scores_embed};
use crate::utils::osu::regex::get_beatmap_info;

/// Display information about your osu! user.
#[poise::command(
    prefix_command,
    slash_command,
    category = "osu!",
    subcommands(
        "link",
        "score",
        "unlink",
        "mode",
        "recent",
        "top",
        "score_notifications",
        "map_notifications",
        "delete_guild_config"
    )
)]
pub async fn osu(
    ctx: Context<'_>,
    #[rest]
    #[description = "User to see profile for."]
    user: Option<User>,
) -> Result<(), Error> {
    let user = user.as_ref().unwrap_or_else(|| ctx.author());
    let connection = &mut ctx.data().db_pool.get()?;
    let profile = linked_osu_profiles::read(connection, user.id.0.get() as i64);
    match profile {
        Ok(profile) => {
            let color: Colour;
            if let Some(guild) = ctx.guild() {
                if let Some(member) = ctx.cache_and_http().cache.member(guild.id, user.id) {
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

            let author = CreateEmbedAuthor::new(&user.name).icon_url(user.face());

            let embed = CreateEmbed::new()
                .image(format!("https://osusig.lolicon.app/sig.php?colour={}&uname={}&pp=0&countryrank=&xpbar=&mode={}&date={}{}",
                               colour_formatted, profile.osu_id, mode, Utc::now().timestamp(), darkheader)).color(color).author(author);

            let builder = CreateReply::default().embed(embed);

            ctx.send(builder).await?;
        }
        Err(_) => {
            ctx.say(format_missing_user_string(ctx, user).await).await?;
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
    user: Option<User>,
) -> Result<(), Error> {
    let user = user.as_ref().unwrap_or_else(|| ctx.author());
    let connection = &mut ctx.data().db_pool.get()?;
    let profile = linked_osu_profiles::read(connection, user.id.0.get() as i64);
    match profile {
        Ok(profile) => {
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
                gamemode_from_string(&profile.mode).unwrap()
            };

            let osu_user = if let Ok(user) = osu_users::read(connection, profile.osu_id) {
                user
            } else {
                ctx.say(
                    "User data hasn't been retrieved for you yet. Please wait a bit and try again",
                )
                .await?;
                return Ok(());
            };

            let score = ctx
                .data()
                .osu_client
                .beatmap_user_score(beatmap_id as u32, profile.osu_id as u32)
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
                        user,
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
        }
        Err(_) => {
            ctx.say(format_missing_user_string(ctx, user).await).await?;
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
    user: Option<User>,
) -> Result<(), Error> {
    let user = user.as_ref().unwrap_or_else(|| ctx.author());
    let connection = &mut ctx.data().db_pool.get()?;
    let profile = linked_osu_profiles::read(connection, user.id.0.get() as i64);
    match profile {
        Ok(profile) => {
            let osu_user = if let Ok(user) = osu_users::read(connection, profile.osu_id) {
                user
            } else {
                ctx.say(
                    "User data hasn't been retrieved for you yet. Please wait a bit and try again",
                )
                .await?;
                return Ok(());
            };

            let recent_score = ctx
                .data()
                .osu_client
                .user_scores(profile.osu_id as u32)
                .recent()
                .mode(gamemode_from_string(&profile.mode).unwrap())
                .include_fails(true)
                .limit(1)
                .await;

            match recent_score {
                Ok(scores) => {
                    if scores.is_empty() {
                        ctx.say(format!("No recent scores found for {}.", user.name))
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

                        send_score_embed(ctx, user, score, &beatmap, &beatmapset, osu_user, None)
                            .await?;
                    }
                }
                Err(why) => {
                    ctx.say(format!("Failed to get recent scores. {}", why))
                        .await?;
                }
            }
        }
        Err(_) => {
            ctx.say(format_missing_user_string(ctx, user).await).await?;
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

/// Display a list of your top scores.
#[poise::command(prefix_command, slash_command, category = "osu!")]
pub async fn top(
    ctx: Context<'_>,
    #[description = "User to see profile for."] user: Option<User>,
    #[description = "Sort your top scores by something other than pp."] sort_type: Option<
        SortChoices,
    >,
) -> Result<(), Error> {
    let user = user.as_ref().unwrap_or_else(|| ctx.author());
    let connection = &mut ctx.data().db_pool.get()?;
    let profile = linked_osu_profiles::read(connection, user.id.0.get() as i64);
    let sort_type = sort_type.unwrap_or(SortChoices::PP);
    match profile {
        Ok(profile) => {
            let osu_user = if let Ok(user) = osu_users::read(connection, profile.osu_id) {
                user
            } else {
                ctx.say(
                    "User data hasn't been retrieved for you yet. Please wait a bit and try again",
                )
                .await?;
                return Ok(());
            };

            let best_scores = ctx
                .data()
                .osu_client
                .user_scores(profile.osu_id as u32)
                .best()
                .mode(gamemode_from_string(&profile.mode).unwrap())
                .limit(100)
                .await;
            match best_scores {
                Ok(api_scores) => {
                    let mut best_scores: Vec<(Score, usize)> = Vec::new();
                    for (pos, score) in api_scores.iter().enumerate() {
                        best_scores.push((score.clone(), pos + 1));
                    }

                    match sort_type {
                        SortChoices::Recent => {
                            best_scores.sort_by(|a, b| b.0.ended_at.cmp(&a.0.ended_at));
                        }
                        SortChoices::Oldest => {
                            best_scores.sort_by(|a, b| a.0.ended_at.cmp(&b.0.ended_at))
                        }
                        SortChoices::Accuracy => {
                            best_scores.sort_by(|a, b| b.0.accuracy.total_cmp(&a.0.accuracy));
                        }
                        SortChoices::Combo => {
                            best_scores.sort_by(|a, b| b.0.max_combo.cmp(&a.0.max_combo))
                        }
                        SortChoices::Score => best_scores.sort_by(|a, b| b.0.score.cmp(&a.0.score)),
                        _ => {}
                    }
                    send_top_scores_embed(ctx, user, connection, &best_scores, osu_user).await?;
                }
                Err(why) => {
                    ctx.say(format!("Failed to get best scores. {}", why))
                        .await?;
                }
            }
        }
        Err(_) => {
            ctx.say(format_missing_user_string(ctx, user).await).await?;
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
