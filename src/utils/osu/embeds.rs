use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::utils::osu::misc::{calculate_potential_acc, count_score_pages};
use crate::utils::osu::misc_format::{format_footer, format_user_link};
use crate::utils::osu::score_format::format_score_list;
use crate::{Context, Error};
use poise::serenity_prelude::model::colour::colours::roles::BLUE;
use poise::serenity_prelude::{
    CacheHttp, Colour, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedAuthor,
    CreateEmbedFooter,
};
use poise::{serenity_prelude, CreateReply, ReplyHandle};
use rosu_v2::prelude::{Score, User};
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
    discord_user: &serenity_prelude::User,
    score: &Score,
    beatmap: &Beatmap,
    beatmapset: &Beatmapset,
    user: User,
    scoreboard_rank: Option<&usize>,
) -> Result<(), Error> {
    let color: Colour;

    let pp =
        crate::utils::osu::calculate::calculate(score, beatmap, calculate_potential_acc(score))
            .await;

    let footer: String;
    let pp = if let Ok(pp) = pp {
        footer = format_footer(score, beatmap, &pp)?;
        Some(pp)
    } else {
        footer = String::new();
        None
    };

    let formatted_score = crate::utils::osu::score_format::format_new_score(
        score,
        beatmap,
        beatmapset,
        &pp,
        scoreboard_rank,
    )?;

    if let Some(guild_ref) = ctx.guild() {
        let guild = guild_ref.clone();
        if let Some(member) = ctx
            .cache()
            .ok_or("Failed to get discord cache in send_score_embed function")?
            .member(guild.id, discord_user.id)
        {
            color = member.colour(ctx.discord()).unwrap_or(BLUE);
        } else {
            color = BLUE;
        }
    } else {
        color = BLUE;
    };

    let embed = create_embed(
        color,
        &beatmapset.list_cover,
        &format!(
            "{}<t:{}:R>",
            formatted_score,
            score.ended_at.unix_timestamp()
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
    discord_user: &serenity_prelude::User,
    best_scores: &[(Score, usize, Beatmap, Beatmapset)],
    user: &User,
    paginate: bool,
    thumbnail: &str,
) -> Result<(), Error> {
    let color: Colour;
    if let Some(guild) = ctx.guild() {
        if let Some(member) = ctx
            .cache()
            .ok_or("Failed to get discord cache in send_scores_embed function")?
            .member(guild.id, discord_user.id)
        {
            color = member.colour(ctx.discord()).unwrap_or(BLUE);
        } else {
            color = BLUE;
        }
    } else {
        color = BLUE;
    };

    let formatted_scores = format_score_list(best_scores, None, None).await?;

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
    best_scores: &[(Score, usize, Beatmap, Beatmapset)],
    color: Colour,
    user: &User,
) -> Result<(), Error> {
    let mut offset: usize = 0;
    let mut page = 1;
    let max_pages = count_score_pages(best_scores.len(), 5);

    while let Some(interaction) = reply
        .message()
        .await?
        .await_component_interaction(&ctx.discord().shard)
        .timeout(Duration::from_secs(15))
        .await
    {
        let choice = &interaction.data.custom_id;
        match choice.as_str() {
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
    best_scores: &[(Score, usize, Beatmap, Beatmapset)],
    offset: usize,
    page: &usize,
    max_pages: &usize,
    color: Colour,
    user: &User,
) -> Result<(), Error> {
    let formatted_scores = format_score_list(best_scores, None, Some(offset)).await?;

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
    best_scores: &[(Score, usize, Beatmap, Beatmapset)],
    offset: usize,
    page: &usize,
    max_pages: &usize,
    color: Colour,
    user: &User,
) -> Result<(), Error> {
    let formatted_scores = format_score_list(best_scores, None, Some(offset)).await?;

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
