use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::utils::osu::misc::count_score_pages;
use crate::utils::osu::misc_format::{format_footer, format_user_link};
use crate::utils::osu::pp::CalculateResults;
use crate::utils::osu::score_format::format_score_list;
use crate::{Context, Error};
use poise::serenity_prelude::model::colour::colours::roles::BLUE;
use poise::serenity_prelude::CreateInteractionResponse::UpdateMessage;
use poise::serenity_prelude::{
    Colour, ComponentInteraction, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedAuthor,
    CreateEmbedFooter, CreateInteractionResponseMessage,
};
use poise::{CreateReply, ReplyHandle};
use rosu_v2::prelude::{Score, UserExtended};
use std::time::Duration;

pub fn create_embed<'a>(
    color: Colour,
    thumbnail: &'a str,
    description: &'a str,
    footer: &'a str,
    author_icon: &'a str,
    author_name: &'a str,
    author_url: &'a str,
) -> CreateEmbed<'a> {
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
        Some(member) => member.colour(ctx.cache()).unwrap_or(BLUE),
    };

    let user_link = format_user_link(i64::from(user.user_id));

    let description = format!(
        "{}<t:{}:R>",
        formatted_score,
        score.0.ended_at.unix_timestamp()
    );

    let embed = create_embed(
        color,
        &score.2.list_cover,
        &description,
        &footer,
        &user.avatar_url,
        &user.username,
        &user_link,
    );

    let builder = CreateReply::default().embed(embed);

    ctx.send(builder).await?;

    Ok(())
}

pub async fn send_scores_embed(
    ctx: Context<'_>,
    best_scores: Vec<(Score, usize, Beatmap, Beatmapset, CalculateResults)>,
    user: &UserExtended,
    thumbnail: &str,
) -> Result<(), Error> {
    let color = match ctx.author_member().await {
        None => BLUE,
        Some(member) => member.colour(ctx.cache()).unwrap_or(BLUE),
    };

    let formatted_scores = format_score_list(&best_scores, None, None)?;

    let user_link = format_user_link(i64::from(user.user_id));

    let footer = format!("Page {} of {}", 1, count_score_pages(best_scores.len(), 5));

    let embed = create_embed(
        color,
        thumbnail,
        &formatted_scores,
        &footer,
        &user.avatar_url,
        &user.username,
        &user_link,
    );

    if best_scores.len() > 5 {
        let components = vec![CreateActionRow::Buttons(vec![
            CreateButton::new("last_page").label("<"),
            CreateButton::new("next_page").label(">"),
            CreateButton::new("reset").label("⭯"),
        ])];

        let builder = CreateReply::default().embed(embed).components(components);

        let reply = ctx.send(builder).await?;

        TopScorePaginator::new(ctx, reply, best_scores, color, user.clone())
            .handle_interactions()
            .await?;
    } else {
        let builder = CreateReply::default().embed(embed);

        ctx.send(builder).await?;
    }

    Ok(())
}

struct TopScorePaginator<'a> {
    ctx: Context<'a>,
    reply: ReplyHandle<'a>,
    best_scores: Vec<(Score, usize, Beatmap, Beatmapset, CalculateResults)>,
    color: Colour,
    user: UserExtended,
    page: usize,
    offset: usize,
    max_pages: usize,
}

impl TopScorePaginator<'_> {
    fn new<'a>(
        ctx: Context<'a>,
        reply: ReplyHandle<'a>,
        best_scores: Vec<(Score, usize, Beatmap, Beatmapset, CalculateResults)>,
        color: Colour,
        user: UserExtended,
    ) -> TopScorePaginator<'a> {
        let max_pages = count_score_pages(best_scores.len(), 5);
        TopScorePaginator {
            ctx,
            reply,
            best_scores,
            color,
            user,
            page: 1,
            offset: 0,
            max_pages,
        }
    }

    async fn handle_interactions(&mut self) -> Result<(), Error> {
        while let Some(interaction) = self
            .reply
            .message()
            .await?
            .await_component_interaction(self.ctx.serenity_context().shard.clone())
            .timeout(Duration::from_secs(15))
            .await
        {
            let choice = &interaction.data.custom_id;
            match choice.as_str() {
                "last_page" => {
                    if self.page == 1 {
                        self.page = self.max_pages;
                        self.offset = (self.max_pages - 1) * 5;
                    } else {
                        self.page -= 1;
                        self.offset -= 5;
                    }
                    self.update_page(&interaction).await?;
                }
                "next_page" => {
                    if self.page == self.max_pages {
                        self.page = 1;
                        self.offset = 0;
                    } else {
                        self.page += 1;
                        self.offset += 5;
                    }
                    self.update_page(&interaction).await?;
                }
                "reset" => {
                    self.page = 1;
                    self.offset = 0;
                    self.update_page(&interaction).await?;
                }
                _ => {}
            };
        }
        self.stop_paginator().await?;
        Ok(())
    }

    async fn update_page(&self, interaction: &ComponentInteraction) -> Result<(), Error> {
        let formatted_scores = format_score_list(&self.best_scores, None, Some(self.offset))?;

        let footer = format!("Page {} of {}", self.page, self.max_pages);

        let user_link = format_user_link(i64::from(self.user.user_id));

        let embed = create_embed(
            self.color,
            &self.user.avatar_url,
            &formatted_scores,
            &footer,
            &self.user.avatar_url,
            &self.user.username,
            &user_link,
        );

        let interaction_response = CreateInteractionResponseMessage::new().embed(embed);

        interaction
            .create_response(self.ctx.http(), UpdateMessage(interaction_response))
            .await?;

        Ok(())
    }

    async fn stop_paginator(&self) -> Result<(), Error> {
        let formatted_scores = format_score_list(&self.best_scores, None, Some(self.offset))?;

        let footer = format!("Page {} of {}", self.page, self.max_pages);

        let user_link = format_user_link(i64::from(self.user.user_id));

        let embed = create_embed(
            self.color,
            &self.user.avatar_url,
            &formatted_scores,
            &footer,
            &self.user.avatar_url,
            &self.user.username,
            &user_link,
        );

        let builder = CreateReply::default().embed(embed).components(vec![]);

        self.reply.edit(self.ctx, builder).await?;

        Ok(())
    }
}
