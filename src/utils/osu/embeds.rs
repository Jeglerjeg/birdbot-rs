use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::models::osu_users::OsuUser;
use crate::serenity_prelude;
use crate::utils::osu::misc::{calculate_potential_acc, count_score_pages};
use crate::utils::osu::misc_format::{
    format_completion_rate, format_potential_string, format_user_link,
};
use crate::utils::osu::score_format::format_score_list;
use crate::{Context, Error};
use diesel::PgConnection;
use humantime::format_duration;
use poise::ReplyHandle;
use rosu_v2::model::{GameMode, Grade};
use rosu_v2::prelude::Score;
use serenity::builder::CreateEmbed;
use serenity::utils::colours::roles::BLUE;
use serenity::utils::Color;
use std::time::Duration;
use time::OffsetDateTime;

pub fn create_embed<'a>(
    f: &'a mut CreateEmbed,
    color: Color,
    thumbnail: &str,
    description: &str,
    footer: &str,
    author_icon: &str,
    author_name: &str,
    author_url: &str,
) -> &'a mut CreateEmbed {
    f.thumbnail(thumbnail)
        .color(color)
        .description(description)
        .footer(|f| f.text(footer))
        .author(|a| a.icon_url(author_icon).name(author_name).url(author_url))
}

pub async fn send_score_embed(
    ctx: Context<'_>,
    score: &Score,
    beatmap: &Beatmap,
    beatmapset: &Beatmapset,
    user: OsuUser,
) -> Result<(), Error> {
    let color: Color;

    let pp =
        crate::utils::osu::calculate::calculate(score, beatmap, calculate_potential_acc(score))
            .await;

    let time_since = format!(
        "\n{} ago",
        format_duration(Duration::new(
            (OffsetDateTime::now_utc() - score.ended_at).as_seconds_f64() as u64,
            0,
        ))
    );

    let potential_string: String;
    let completion_rate: String;
    let pp = if let Ok(pp) = pp {
        potential_string = format_potential_string(&pp);
        if score.grade == Grade::F && score.mode != GameMode::Catch {
            completion_rate = format!("\n{}", format_completion_rate(score, beatmap, &pp));
        } else {
            completion_rate = String::new();
        }
        Some(pp)
    } else {
        potential_string = String::new();
        completion_rate = String::new();
        None
    };

    let footer = format!("{}{}{}", potential_string, time_since, completion_rate);

    let formatted_score =
        crate::utils::osu::score_format::format_new_score(score, beatmap, beatmapset, &pp);

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
            create_embed(
                e,
                color,
                &beatmapset.list_cover,
                &formatted_score,
                &footer,
                &user.avatar_url,
                &user.username,
                &format_user_link(user.id),
            )
        })
    })
    .await?;

    Ok(())
}

pub async fn send_top_scores_embed(
    ctx: Context<'_>,
    connection: &mut PgConnection,
    best_scores: &[Score],
    user: OsuUser,
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

    let formatted_scores = format_score_list(
        connection,
        ctx.data().osu_client.clone(),
        best_scores,
        None,
        None,
    )
    .await?;

    let reply = ctx
        .send(|m| {
            m.embed(|e| {
                create_embed(
                    e,
                    color,
                    &user.avatar_url,
                    &formatted_scores,
                    &format!("Page {} of {}", 1, count_score_pages(best_scores, 5)),
                    &user.avatar_url,
                    user.username.as_str(),
                    &format_user_link(user.id),
                )
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

    handle_top_score_interactions(ctx, connection, reply, best_scores, color, &user).await?;

    Ok(())
}

async fn handle_top_score_interactions(
    ctx: Context<'_>,
    connection: &mut PgConnection,
    reply: ReplyHandle<'_>,
    best_scores: &[Score],
    color: Color,
    user: &OsuUser,
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
                    connection,
                    &reply,
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
                    connection,
                    &reply,
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
                    connection,
                    &reply,
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
        connection,
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
    connection: &mut PgConnection,
    reply: ReplyHandle<'_>,
    best_scores: &[Score],
    offset: usize,
    page: &usize,
    max_pages: &usize,
    color: Color,
    user: &OsuUser,
) -> Result<(), Error> {
    let formatted_scores = format_score_list(
        connection,
        ctx.data().osu_client.clone(),
        best_scores,
        None,
        Some(offset),
    )
    .await?;
    reply
        .edit(ctx, |b| {
            b.embed(|e| {
                create_embed(
                    e,
                    color,
                    &user.avatar_url,
                    &formatted_scores,
                    &format!("Page {} of {}", page, max_pages),
                    &user.avatar_url,
                    user.username.as_str(),
                    &format_user_link(user.id),
                )
            })
            .components(|b| b)
        })
        .await?;

    Ok(())
}

async fn change_top_scores_page(
    ctx: Context<'_>,
    connection: &mut PgConnection,
    reply: &ReplyHandle<'_>,
    best_scores: &[Score],
    offset: usize,
    page: &usize,
    max_pages: &usize,
    color: Color,
    user: &OsuUser,
) -> Result<(), Error> {
    let formatted_scores = format_score_list(
        connection,
        ctx.data().osu_client.clone(),
        best_scores,
        None,
        Some(offset),
    )
    .await?;

    reply
        .edit(ctx, |b| {
            b.embed(|e| {
                create_embed(
                    e,
                    color,
                    &user.avatar_url,
                    &formatted_scores,
                    &format!("Page {} of {}", page, max_pages),
                    &user.avatar_url,
                    user.username.as_str(),
                    &format_user_link(user.id),
                )
            })
        })
        .await?;

    Ok(())
}
