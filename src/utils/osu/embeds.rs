use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::models::linked_osu_profiles::LinkedOsuProfile;
use crate::serenity_prelude;
use crate::utils::osu::misc::{calculate_potential_acc, count_score_pages};
use crate::utils::osu::misc_format::{
    format_completion_rate, format_potential_string, format_user_link,
};
use crate::utils::osu::score_format::format_score_list;
use crate::{Context, Error};
use humantime::format_duration;
use poise::ReplyHandle;
use rosu_v2::model::{GameMode, Grade};
use rosu_v2::prelude::{Score, User};
use serenity::model::prelude::interaction::message_component::MessageComponentInteraction;
use serenity::utils::colours::roles::BLUE;
use serenity::utils::Color;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;

pub async fn send_score_embed(
    ctx: Context<'_>,
    score: Score,
    beatmap: Beatmap,
    beatmapset: Beatmapset,
    user: User,
) -> Result<(), Error> {
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
    let completion_rate: String;
    let pp = if let Ok(pp) = pp {
        potential_string = format_potential_string(&pp);
        if score.grade == Grade::F && score.mode != GameMode::Catch {
            completion_rate = format!("\n{}", format_completion_rate(&score, &beatmap, &pp));
        } else {
            completion_rate = String::new();
        }
        Some(pp)
    } else {
        potential_string = String::new();
        completion_rate = String::new();
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
                .footer(|f| f.text(potential_string + &*time_since + &*completion_rate))
                .author(|a| {
                    a.icon_url(user.avatar_url)
                        .name(user.username)
                        .url(format_user_link(&user.user_id))
                })
        })
    })
    .await?;

    Ok(())
}

pub async fn send_top_scores_embed(
    ctx: Context<'_>,
    best_scores: &[Score],
    profile: LinkedOsuProfile,
) -> Result<(), Error> {
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

    let reply = ctx
        .send(|m| {
            m.embed(|e| {
                e.description(formatted_scores)
                    .thumbnail(&user.avatar_url)
                    .color(color)
                    .author(|a| {
                        a.name(&user.username.as_str())
                            .icon_url(&user.avatar_url)
                            .url(format_user_link(&user.user_id))
                    })
                    .footer(|f| {
                        f.text(format!(
                            "Page {} of {}",
                            1,
                            count_score_pages(best_scores, 5)
                        ))
                    })
            })
            .components(|c| {
                c.create_action_row(|r| {
                    r.create_button(|b| {
                        b.custom_id("last_page")
                            .label("<")
                            .style(serenity_prelude::ButtonStyle::Primary)
                    })
                    .create_button(|b| {
                        b.custom_id("next_page")
                            .label(">")
                            .style(serenity_prelude::ButtonStyle::Primary)
                    })
                    .create_button(|b| {
                        b.custom_id("reset")
                            .label("⭯")
                            .style(serenity_prelude::ButtonStyle::Primary)
                    })
                })
            })
        })
        .await?;

    handle_top_score_interactions(ctx, reply, best_scores, color, &user).await?;

    Ok(())
}

async fn handle_top_score_interactions(
    ctx: Context<'_>,
    reply: ReplyHandle<'_>,
    best_scores: &[Score],
    color: Color,
    user: &User,
) -> Result<(), Error> {
    let mut offset: usize = 0;
    let mut page = 1;
    let max_pages = count_score_pages(best_scores, 5);

    loop {
        let interaction = match reply
            .message()
            .await?
            .await_component_interaction(ctx.discord())
            .timeout(Duration::from_secs(15))
            .await
        {
            Some(x) => x,
            None => {
                break;
            }
        };

        let choice = &interaction.data.custom_id;
        match &**choice {
            "last_page" => {
                interaction.defer(ctx.discord()).await?;
                if page == 1 {
                    page = max_pages;
                    offset = (max_pages - 1) * 5;
                } else {
                    page -= 1;
                    offset -= 5;
                }
                change_top_scores_page(
                    ctx,
                    interaction,
                    best_scores,
                    offset,
                    &page,
                    &max_pages,
                    color,
                    user,
                )
                .await?;
            }
            "next_page" => {
                interaction.defer(ctx.discord()).await?;
                if page == max_pages {
                    page = 1;
                    offset = 0;
                } else {
                    page += 1;
                    offset += 5;
                }
                change_top_scores_page(
                    ctx,
                    interaction,
                    best_scores,
                    offset,
                    &page,
                    &max_pages,
                    color,
                    user,
                )
                .await?;
            }
            "reset" => {
                interaction.defer(ctx.discord()).await?;
                page = 1;
                offset = 0;
                change_top_scores_page(
                    ctx,
                    interaction,
                    best_scores,
                    offset,
                    &page,
                    &max_pages,
                    color,
                    user,
                )
                .await?;
            }
            _ => {}
        };
    }

    remove_top_score_paginators(
        ctx,
        reply,
        best_scores,
        offset,
        &page,
        &max_pages,
        color,
        user,
    )
    .await?;

    Ok(())
}

async fn remove_top_score_paginators(
    ctx: Context<'_>,
    reply: ReplyHandle<'_>,
    best_scores: &[Score],
    offset: usize,
    page: &usize,
    max_pages: &usize,
    color: Color,
    user: &User,
) -> Result<(), Error> {
    let formatted_scores = format_score_list(ctx, best_scores, None, Some(offset)).await?;
    reply
        .into_message()
        .await?
        .edit(ctx.discord(), |b| {
            b.components(|b| b);
            b.embed(|e| {
                e.description(formatted_scores)
                    .thumbnail(&user.avatar_url)
                    .color(color)
                    .author(|a| {
                        a.name(&user.username.as_str())
                            .icon_url(&user.avatar_url)
                            .url(format_user_link(&user.user_id))
                    })
                    .footer(|f| f.text(format!("Page {} of {}", page, max_pages)))
            })
        })
        .await?;

    Ok(())
}

async fn change_top_scores_page(
    ctx: Context<'_>,
    interaction: Arc<MessageComponentInteraction>,
    best_scores: &[Score],
    offset: usize,
    page: &usize,
    max_pages: &usize,
    color: Color,
    user: &User,
) -> Result<(), Error> {
    let formatted_scores = format_score_list(ctx, best_scores, None, Some(offset)).await?;

    interaction
        .message
        .clone()
        .edit(ctx.discord(), |b| {
            b.embed(|e| {
                e.description(formatted_scores)
                    .thumbnail(&user.avatar_url)
                    .color(color)
                    .author(|a| {
                        a.name(&user.username.as_str())
                            .icon_url(&user.avatar_url)
                            .url(format_user_link(&user.user_id))
                    })
                    .footer(|f| f.text(format!("Page {} of {}", page, max_pages)))
            })
        })
        .await?;

    Ok(())
}
