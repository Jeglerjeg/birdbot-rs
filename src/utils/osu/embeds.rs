use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::utils::osu::misc::count_score_pages;
use crate::utils::osu::misc_format::{format_footer, format_user_link};
use crate::utils::osu::pp::CalculateResults;
use crate::utils::osu::score_format::format_score_list;
use crate::{Context, Error};
use poise::serenity_prelude::model::colour::colours::roles::BLUE;
use poise::serenity_prelude::{
    Colour, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter,
};
use poise::{CreateReply, ReplyHandle};
use rosu_v2::prelude::{Score, UserExtended};
use std::time::Duration;

pub fn create_embed(
    color: Colour,
    thumbnail: &str,
    description: &str,
    footer: &str,
    author_icon: &str,
    author_name: &str,
    author_url: &str,
) -> CreateEmbed {
    let embed = CreateEmbed::new();

    let created_footer = CreateEmbedFooter::new(footer);

    let created_author = CreateEmbedAuthor::new(author_name)
        .icon_url(author_icon)
        .url(author_url);

    embed
        .thumbnail(thumbnail)
        .color(color)
        .description(description)
        .footer(created_footer)
        .author(created_author)
}

pub async fn send_score_embed(
    ctx: Context<'_>,
    score: (&Score, &Beatmap, &Beatmapset, &CalculateResults),
    user: UserExtended,
    scoreboard_rank: Option<&usize>,
) -> Result<(), Error> {
    let footer = format_footer(score.0, score.1, score.3)?;

    let formatted_score = crate::utils::osu::score_format::format_new_score(
        score.0,
        score.1,
        score.2,
        score.3,
        scoreboard_rank,
    )?;

    let color = match ctx.author_member().await {
        None => BLUE,
        Some(member) => member.colour(ctx).unwrap_or(BLUE),
    };

    let embed = create_embed(
        color,
        &score.2.list_cover,
        &format!(
            "{}<t:{}:R>",
            formatted_score,
            score.0.ended_at.unix_timestamp()
        ),
        &footer,
        &user.avatar_url,
        &user.username,
        &format_user_link(i64::from(user.user_id)),
    );

    let builder = CreateReply::default().embed(embed);

    ctx.send(builder).await?;

    Ok(())
}

pub async fn send_scores_embed(
    ctx: Context<'_>,
    best_scores: &[(Score, usize, Beatmap, Beatmapset, CalculateResults)],
    user: &UserExtended,
    paginate: bool,
    thumbnail: &str,
) -> Result<(), Error> {
    let color = match ctx.author_member().await {
        None => BLUE,
        Some(member) => member.colour(ctx).unwrap_or(BLUE),
    };

    let formatted_scores = format_score_list(best_scores, None, None)?;

    let embed = create_embed(
        color,
        thumbnail,
        &formatted_scores,
        &format!("Page {} of {}", 1, count_score_pages(best_scores.len(), 5)),
        &user.avatar_url,
        user.username.as_str(),
        &format_user_link(i64::from(user.user_id)),
    );

    if paginate {
        let components = vec![CreateActionRow::Buttons(vec![
            CreateButton::new("last_page").label("<"),
            CreateButton::new("next_page").label(">"),
            CreateButton::new("reset").label("⭯"),
        ])];

        let builder = CreateReply::new().embed(embed).components(components);

        let reply = ctx.send(builder).await?;

        handle_top_score_interactions(ctx, reply, best_scores, color, user).await?;
    } else {
        let builder = CreateReply::new().embed(embed);

        ctx.send(builder).await?;
    }

    Ok(())
}

async fn handle_top_score_interactions(
    ctx: Context<'_>,
    reply: ReplyHandle<'_>,
    best_scores: &[(Score, usize, Beatmap, Beatmapset, CalculateResults)],
    color: Colour,
    user: &UserExtended,
) -> Result<(), Error> {
    let mut offset: usize = 0;
    let mut page = 1;
    let max_pages = count_score_pages(best_scores.len(), 5);

    while let Some(interaction) = reply
        .message()
        .await?
        .await_component_interaction(ctx)
        .timeout(Duration::from_secs(15))
        .await
    {
        let choice = &interaction.data.custom_id;
        match choice.as_str() {
            "last_page" => {
                interaction.defer(ctx).await?;
                if page == 1 {
                    page = max_pages;
                    offset = (max_pages - 1) * 5;
                } else {
                    page -= 1;
                    offset -= 5;
                }
                change_top_scores_page(
                    ctx,
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
                interaction.defer(ctx).await?;
                if page == max_pages {
                    page = 1;
                    offset = 0;
                } else {
                    page += 1;
                    offset += 5;
                }
                change_top_scores_page(
                    ctx,
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
                interaction.defer(ctx).await?;
                page = 1;
                offset = 0;
                change_top_scores_page(
                    ctx,
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
    best_scores: &[(Score, usize, Beatmap, Beatmapset, CalculateResults)],
    offset: usize,
    page: &usize,
    max_pages: &usize,
    color: Colour,
    user: &UserExtended,
) -> Result<(), Error> {
    let formatted_scores = format_score_list(best_scores, None, Some(offset))?;

    let embed = create_embed(
        color,
        &user.avatar_url,
        &formatted_scores,
        &format!("Page {page} of {max_pages}"),
        &user.avatar_url,
        user.username.as_str(),
        &format_user_link(i64::from(user.user_id)),
    );

    let builder = CreateReply::default().embed(embed).components(vec![]);

    reply.edit(ctx, builder).await?;

    Ok(())
}

async fn change_top_scores_page(
    ctx: Context<'_>,
    reply: &ReplyHandle<'_>,
    best_scores: &[(Score, usize, Beatmap, Beatmapset, CalculateResults)],
    offset: usize,
    page: &usize,
    max_pages: &usize,
    color: Colour,
    user: &UserExtended,
) -> Result<(), Error> {
    let formatted_scores = format_score_list(best_scores, None, Some(offset))?;

    let embed = create_embed(
        color,
        &user.avatar_url,
        &formatted_scores,
        &format!("Page {page} of {max_pages}"),
        &user.avatar_url,
        user.username.as_str(),
        &format_user_link(i64::from(user.user_id)),
    );

    let builder = CreateReply::default().embed(embed);

    reply.edit(ctx, builder).await?;

    Ok(())
}
