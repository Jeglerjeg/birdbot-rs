use crate::models::linked_osu_profiles::NewLinkedOsuProfile;
use crate::utils::db::linked_osu_profiles;
use crate::utils::osu::misc::gamemode_from_string;
use crate::utils::osu::misc_format::format_missing_user_string;
use chrono::Utc;
use serenity::utils::colours::roles::BLUE;
use serenity::utils::Color;

use crate::{Context, Error};

use crate::utils::osu::embeds::{send_score_embed, send_top_scores_embed};

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

            ctx.send(|m|
                m.embed(|e|
                    e.image(format!("https://lemmmy.pw/osusig//sig.php?colour={}&uname={}&countryrank=&xpbar=&mode={}&date={}{}",
                                    colour_formatted, profile.osu_id, mode, Utc::now().timestamp(), darkheader))
                        .author(|a| a.icon_url(ctx.author().face()).name(&ctx.author().name))
                        .color(color)))
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

                    send_score_embed(ctx, score.score, beatmap, beatmapset, user).await?;
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

                        send_score_embed(ctx, score, beatmap, beatmapset, user).await?;
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
pub async fn top(
    ctx: Context<'_>,
    #[description = "Sort your top scores by something other than pp."] sort_type: Option<String>,
) -> Result<(), Error> {
    let profile = linked_osu_profiles::read(ctx.author().id.0 as i64);
    let sort_type = sort_type.unwrap_or_default();
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
                Ok(mut best_scores) => {
                    match sort_type.as_str() {
                        "newest" | "recent" => {
                            best_scores.sort_by(|a, b| b.ended_at.cmp(&a.ended_at));
                        }
                        "oldest" => best_scores.sort_by(|a, b| a.ended_at.cmp(&b.ended_at)),
                        "acc" | "accuracy" => {
                            best_scores.sort_by(|a, b| b.accuracy.total_cmp(&a.accuracy));
                        }
                        "combo" => best_scores.sort_by(|a, b| b.max_combo.cmp(&a.max_combo)),
                        "score" => best_scores.sort_by(|a, b| b.score.cmp(&a.score)),
                        _ => {}
                    }
                    send_top_scores_embed(ctx, &best_scores, profile).await?;
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
