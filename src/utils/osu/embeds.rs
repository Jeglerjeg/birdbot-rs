use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::models::osu_users::OsuUser;
use crate::utils::osu::misc::{calculate_potential_acc, count_score_pages};
use crate::utils::osu::misc_format::{
    format_completion_rate, format_potential_string, format_user_link,
};
use crate::utils::osu::score_format::format_score_list;
use crate::{Context, Error};
use diesel::PgConnection;
use poise::{serenity_prelude, CreateReply, ReplyHandle};
use rosu_v2::model::{GameMode, Grade};
use rosu_v2::prelude::Score;
use serenity_prelude::model::colour::colours::roles::BLUE;
use serenity_prelude::{
    Colour, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter,
};
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
    score: &Score,
    beatmap: &Beatmap,
    beatmapset: &Beatmapset,
    user: OsuUser,
) -> Result<(), Error> {
    let color: Colour;

    let pp =
        crate::utils::osu::calculate::calculate(score, beatmap, calculate_potential_acc(score))
            .await;

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

    let footer = format!("{}{}", potential_string, completion_rate);

    let formatted_score =
        crate::utils::osu::score_format::format_new_score(score, beatmap, beatmapset, &pp);

    if let Some(guild_ref) = ctx.guild() {
        let guild = guild_ref.clone();
        if let Some(member) = ctx.cache_and_http().cache.member(guild.id, ctx.author().id) {
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
        &formatted_score,
        &footer,
        &user.avatar_url,
        &user.username,
        &format_user_link(user.id),
    );

    let builder = CreateReply::default().embed(embed);

    ctx.send(builder).await?;

    Ok(())
}

pub async fn send_top_scores_embed(
    ctx: Context<'_>,
    connection: &mut PgConnection,
    best_scores: &[(Score, usize)],
    user: OsuUser,
) -> Result<(), Error> {
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

    let formatted_scores = format_score_list(
        connection,
        ctx.data().osu_client.clone(),
        best_scores,
        None,
        None,
    )
    .await?;

    let embed = create_embed(
        color,
        &user.avatar_url,
        &formatted_scores,
        &format!("Page {} of {}", 1, count_score_pages(best_scores, 5)),
        &user.avatar_url,
        user.username.as_str(),
        &format_user_link(user.id),
    );

    let components = vec![CreateActionRow::Buttons(vec![
        CreateButton::new("<", serenity_prelude::ButtonStyle::Primary, "last_page"),
        CreateButton::new(">", serenity_prelude::ButtonStyle::Primary, "next_page"),
        CreateButton::new("⭯", serenity_prelude::ButtonStyle::Primary, "reset"),
    ])];

    let builder = CreateReply::default().embed(embed).components(components);

    let reply = ctx.send(builder).await?;

    handle_top_score_interactions(ctx, connection, reply, best_scores, color, &user).await?;

    Ok(())
}

async fn handle_top_score_interactions(
    ctx: Context<'_>,
    connection: &mut PgConnection,
    reply: ReplyHandle<'_>,
    best_scores: &[(Score, usize)],
    color: Colour,
    user: &OsuUser,
) -> Result<(), Error> {
    let mut offset: usize = 0;
    let mut page = 1;
    let max_pages = count_score_pages(best_scores, 5);

    loop {
        let interaction = match reply
            .message()
            .await?
            .component_interaction_collector(&ctx.discord().shard)
            .timeout(Duration::from_secs(15))
            .collect_single()
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
    best_scores: &[(Score, usize)],
    offset: usize,
    page: &usize,
    max_pages: &usize,
    color: Colour,
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

    let embed = create_embed(
        color,
        &user.avatar_url,
        &formatted_scores,
        &format!("Page {} of {}", page, max_pages),
        &user.avatar_url,
        user.username.as_str(),
        &format_user_link(user.id),
    );

    let builder = CreateReply::default().embed(embed).components(vec![]);

    reply.edit(ctx, builder).await?;

    Ok(())
}

async fn change_top_scores_page(
    ctx: Context<'_>,
    connection: &mut PgConnection,
    reply: &ReplyHandle<'_>,
    best_scores: &[(Score, usize)],
    offset: usize,
    page: &usize,
    max_pages: &usize,
    color: Colour,
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

    let embed = create_embed(
        color,
        &user.avatar_url,
        &formatted_scores,
        &format!("Page {} of {}", page, max_pages),
        &user.avatar_url,
        user.username.as_str(),
        &format_user_link(user.id),
    );

    let builder = CreateReply::default().embed(embed);

    reply.edit(ctx, builder).await?;

    Ok(())
}
