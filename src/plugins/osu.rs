use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::models::linked_osu_profiles::NewLinkedOsuProfile;
use crate::utils::db::linked_osu_profiles;
use crate::utils::osu::misc::{calculate_potential_acc, gamemode_from_string};
use crate::utils::osu::misc_format::{
    format_missing_user_string, format_potential_string, format_user_link,
};
use crate::utils::osu::score_format::format_score_list;
use crate::{Context, Error};
use humantime::format_duration;
use rosu_v2::prelude::{Score, User};
use serenity::utils::colours::roles::BLUE;
use serenity::utils::Color;
use std::time::Duration;
use time::OffsetDateTime;

async fn send_score_embed(
    ctx: Context<'_>,
    score: Score,
    beatmap: Beatmap,
    beatmapset: Beatmapset,
    user: User,
) {
    let color: Color;

    let pp =
        crate::utils::osu::calculate::calculate(&score, &beatmap, calculate_potential_acc(&score))
            .await;

    let time_since = format!(
        "\n{} ago",
        format_duration(Duration::new(
            (OffsetDateTime::now_utc() - score.ended_at)
                .as_seconds_f64()
                .round() as u64,
            0,
        ))
    );

    let potential_string: String;
    let pp = if let Ok(pp) = pp {
        potential_string = format_potential_string(&pp);
        Some(pp)
    } else {
        potential_string = String::new();
        None
    };

    let formatted_score =
        crate::utils::osu::score_format::format_new_score(&score, &beatmap, &beatmapset, &pp);

    if let Some(guild) = ctx.guild() {
        if let Ok(member) = guild.member(ctx.discord(), ctx.author().id).await {
            color = member.colour(ctx.discord()).unwrap_or(BLUE);
        } else {
            color = BLUE;
        }
    } else {
        color = BLUE;
    };

    ctx.send(|m| {
        m.embed(|e| {
            e.thumbnail(beatmapset.list_cover)
                .color(color)
                .description(formatted_score)
                .footer(|f| f.text(potential_string + &*time_since))
                .author(|a| {
                    a.icon_url(user.avatar_url)
                        .name(user.username)
                        .url(format_user_link(&user.user_id))
                })
        })
    })
    .await
    .expect("Couldn't send score embed.");
}

/// Display information about your osu! user.
#[poise::command(
    prefix_command,
    slash_command,
    category = "osu!",
    subcommands("link", "score", "unlink", "mode", "recent", "top")
)]
pub async fn osu(ctx: Context<'_>) -> Result<(), Error> {
    let profile = linked_osu_profiles::read(ctx.author().id.0 as i64);
    match profile {
        Ok(profile) => {
            ctx.say(format!("Your profile is `{}`.", profile.osu_id))
                .await?;
        }
        Err(_) => {
            ctx.say(format_missing_user_string(ctx).await).await?;
        }
    }

    Ok(())
}

/// Link an osu! profile.
#[poise::command(prefix_command, slash_command, guild_only, category = "osu!")]
pub async fn link(
    ctx: Context<'_>,
    #[description = "osu! username to link to"] username: String,
) -> Result<(), Error> {
    let user = ctx.data().osu_client.user(username).await?;

    let query_item = NewLinkedOsuProfile {
        id: ctx.author().id.0 as i64,
        osu_id: i64::from(user.user_id),
        home_guild: ctx.guild_id().unwrap().0 as i64,
        mode: user.mode.to_string(),
    };

    linked_osu_profiles::create(&query_item);

    ctx.say(format!(
        "Set your osu! profile to `{}`.",
        user.username.as_str()
    ))
    .await?;

    Ok(())
}

/// Unlink your osu! profile.
#[poise::command(prefix_command, slash_command, guild_only, category = "osu!")]
pub async fn unlink(ctx: Context<'_>) -> Result<(), Error> {
    let profile = linked_osu_profiles::read(ctx.author().id.0 as i64);

    match profile {
        Ok(profile) => {
            linked_osu_profiles::delete(profile.id).expect("Failed to delete profile");
            ctx.say("Unlinked your profile.").await?;
        }
        Err(_) => {
            ctx.say(format_missing_user_string(ctx).await).await?;
        }
    };

    Ok(())
}

/// Changed your osu! mode.
#[poise::command(prefix_command, slash_command, category = "osu!")]
pub async fn mode(
    ctx: Context<'_>,
    #[description = "Gamemode to switch to."] mode: String,
) -> Result<(), Error> {
    let profile = linked_osu_profiles::read(ctx.author().id.0 as i64);
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

            linked_osu_profiles::update(profile.id, &query_item);

            ctx.say(format!("Updated your osu! mode to {}.", parsed_mode))
                .await?;
        }
        Err(_) => {
            ctx.say(format_missing_user_string(ctx).await).await?;
        }
    }

    Ok(())
}

/// Display your score on a beatmap.
#[poise::command(prefix_command, slash_command, category = "osu!")]
pub async fn score(
    ctx: Context<'_>,
    #[description = "Beatmap ID to check for a score."] beatmap_id: u32,
) -> Result<(), Error> {
    let profile = linked_osu_profiles::read(ctx.author().id.0 as i64);
    match profile {
        Ok(profile) => {
            let score = ctx
                .data()
                .osu_client
                .beatmap_user_score(beatmap_id, profile.osu_id as u32)
                .mode(gamemode_from_string(&profile.mode).unwrap())
                .await;
            match score {
                Ok(score) => {
                    let beatmap = crate::utils::osu::caching::get_beatmap(
                        ctx,
                        score.score.map.as_ref().unwrap().map_id,
                    )
                    .await?;

                    let beatmapset = crate::utils::osu::caching::get_beatmapset(
                        ctx,
                        beatmap.beatmapset_id as u32,
                    )
                    .await?;

                    let user = ctx.data().osu_client.user(profile.osu_id as u32).await?;

                    send_score_embed(ctx, score.score, beatmap, beatmapset, user).await;
                }
                Err(why) => {
                    ctx.say(format!("Failed to get beatmap score. {}", why))
                        .await?;
                }
            }
        }
        Err(_) => {
            ctx.say(format_missing_user_string(ctx).await).await?;
        }
    }

    Ok(())
}

/// Display your most recent osu score.
#[poise::command(prefix_command, slash_command, category = "osu!")]
pub async fn recent(ctx: Context<'_>) -> Result<(), Error> {
    let profile = linked_osu_profiles::read(ctx.author().id.0 as i64);
    match profile {
        Ok(profile) => {
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
                        ctx.say(format!("No recent scores found for {}.", ctx.author().name))
                            .await?;
                    } else {
                        let score = scores[0].clone();

                        let beatmap = crate::utils::osu::caching::get_beatmap(
                            ctx,
                            score.map.as_ref().unwrap().map_id,
                        )
                        .await?;

                        let beatmapset = crate::utils::osu::caching::get_beatmapset(
                            ctx,
                            beatmap.beatmapset_id as u32,
                        )
                        .await?;

                        let user = ctx.data().osu_client.user(profile.osu_id as u32).await?;

                        send_score_embed(ctx, score, beatmap, beatmapset, user).await;
                    }
                }
                Err(why) => {
                    ctx.say(format!("Failed to get recent scores. {}", why))
                        .await?;
                }
            }
        }
        Err(_) => {
            ctx.say(format_missing_user_string(ctx).await).await?;
        }
    }

    Ok(())
}

/// Display a list of your top scores.
#[poise::command(prefix_command, slash_command, category = "osu!")]
pub async fn top(ctx: Context<'_>) -> Result<(), Error> {
    let profile = linked_osu_profiles::read(ctx.author().id.0 as i64);
    match profile {
        Ok(profile) => {
            let best_scores = ctx
                .data()
                .osu_client
                .user_scores(profile.osu_id as u32)
                .best()
                .mode(gamemode_from_string(&profile.mode).unwrap())
                .limit(100)
                .await;

            match best_scores {
                Ok(best_scores) => {
                    let color: Color;
                    if let Some(guild) = ctx.guild() {
                        if let Ok(member) = guild.member(ctx.discord(), ctx.author().id).await {
                            color = member.colour(ctx.discord()).unwrap_or(BLUE);
                        } else {
                            color = BLUE;
                        }
                    } else {
                        color = BLUE;
                    };

                    let formatted_scores = format_score_list(ctx, best_scores, None, None).await?;

                    let user = ctx.data().osu_client.user(profile.osu_id as u32).await?;

                    ctx.send(|m| {
                        m.embed(|e| {
                            e.description(formatted_scores)
                                .thumbnail(&user.avatar_url)
                                .color(color)
                                .author(|a| {
                                    a.name(&user.username.as_str())
                                        .icon_url(&user.avatar_url)
                                        .url(format_user_link(&user.user_id))
                                })
                        })
                    })
                    .await?;
                }
                Err(why) => {
                    ctx.say(format!("Failed to get best scores. {}", why))
                        .await?;
                }
            }
        }
        Err(_) => {
            ctx.say(format_missing_user_string(ctx).await).await?;
        }
    }

    Ok(())
}
