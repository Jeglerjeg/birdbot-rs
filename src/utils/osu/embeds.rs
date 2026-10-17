use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::utils::db::linked_osu_profiles;
use crate::utils::osu::misc::count_score_pages;
use crate::utils::osu::misc_format::{format_beatmap_link, format_footer, format_user_link};
use crate::utils::osu::pp::CalculateResults;
use crate::utils::osu::score_format::{format_minimal_score, format_score_list};
use crate::{Context, Error};
use poise::serenity_prelude::CreateInteractionResponse::UpdateMessage;
use poise::serenity_prelude::model::colour::colours::roles::BLUE;
use poise::serenity_prelude::{
    CollectComponentInteractions, Colour, ComponentInteraction, CreateActionRow, CreateButton,
    CreateComponent, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter,
    CreateInteractionResponseMessage,
};
use poise::{CreateReply, ReplyHandle};
use rosu_v2::prelude::{Score, UserExtended};
use std::time::Duration;
use tracing::log::info;

pub struct OsuEmbedData<'a> {
    pub color: Colour,
    pub thumbnail: &'a str,
    pub description: &'a str,
    pub footer: &'a str,
    pub author_icon: &'a str,
    pub author_name: &'a str,
    pub author_url: &'a str,
    pub title: Option<String>,
    pub title_url: Option<String>,
}

pub fn create_embed(osu_embed: OsuEmbedData) -> CreateEmbed {
    info!("{}", osu_embed.description);
    let embed = CreateEmbed::new();

    let created_footer = CreateEmbedFooter::new(osu_embed.footer);

    let created_author = CreateEmbedAuthor::new(osu_embed.author_name)
        .icon_url(osu_embed.author_icon)
        .url(osu_embed.author_url);

    if let (Some(title), Some(title_url)) = (osu_embed.title, osu_embed.title_url) {
        embed
            .thumbnail(osu_embed.thumbnail, Some("The osu map's background".into()))
            .color(osu_embed.color)
            .description(osu_embed.description)
            .footer(created_footer)
            .author(created_author)
            .title(title)
            .url(title_url)
    } else {
        embed
            .thumbnail(osu_embed.thumbnail, Some("The osu map's background".into()))
            .color(osu_embed.color)
            .description(osu_embed.description)
            .footer(created_footer)
            .author(created_author)
    }
}

pub async fn send_score_embed(
    ctx: Context<'_>,
    score: (&Score, &Beatmap, &Beatmapset, &CalculateResults),
    user: UserExtended,
    scoreboard_rank: Option<&usize>,
    minimal: bool,
) -> Result<(), Error> {
    let footer = format_footer(score.0, score.1, score.3)?;

    let formatted_score = if minimal {
        format_minimal_score(
            score.0,
            score.1,
            score.2,
            score.3,
            false,
            scoreboard_rank,
            None,
        )?
    } else {
        crate::utils::osu::score_format::format_new_score(
            score.0,
            score.1,
            score.2,
            score.3,
            false,
            scoreboard_rank,
            None,
        )?
    };

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

    let title = format!(
        "{} - {} [{}]",
        score.2.artist, score.2.title, score.1.version,
    );

    let title_url = format_beatmap_link(
        Some(score.1.id),
        score.2.id,
        Some(&score.0.mode.to_string()),
    );

    let embed = create_embed(OsuEmbedData {
        color,
        thumbnail: &score.2.list_cover,
        description: &description,
        footer: &footer,
        author_icon: &user.avatar_url,
        author_name: &user.username,
        author_url: &user_link,
        title: Some(title),
        title_url: Some(title_url),
    });

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

    let connection = &mut ctx.data().db_pool.get().await?;

    let minimal_formatting = if let Ok(profile) =
        linked_osu_profiles::read(connection, i64::try_from(ctx.author().id.get())?).await
    {
        profile.minimal_formatting
    } else {
        false
    };

    let formatted_scores = format_score_list(&best_scores, None, None, minimal_formatting)?;

    let user_link = format_user_link(i64::from(user.user_id));

    let footer = format!("Page {} of {}", 1, count_score_pages(best_scores.len(), 5));

    let embed = create_embed(OsuEmbedData {
        color,
        thumbnail,
        description: &formatted_scores,
        footer: &footer,
        author_icon: &user.avatar_url,
        author_name: &user.username,
        author_url: &user_link,
        title: None,
        title_url: None,
    });

    if best_scores.len() > 5 {
        let buttons = [
            CreateButton::new("last_page").label("<"),
            CreateButton::new("next_page").label(">"),
            CreateButton::new("reset").label("⭯"),
        ];

        let components = [CreateComponent::ActionRow(CreateActionRow::buttons(
            &buttons,
        ))];

        let builder = CreateReply::default().embed(embed).components(&components);

        let reply = ctx.send(builder).await?;

        TopScorePaginator::new(
            ctx,
            reply,
            best_scores,
            color,
            user.clone(),
            minimal_formatting,
        )
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
    minimal_formatting: bool,
}

impl TopScorePaginator<'_> {
    fn new<'a>(
        ctx: Context<'a>,
        reply: ReplyHandle<'a>,
        best_scores: Vec<(Score, usize, Beatmap, Beatmapset, CalculateResults)>,
        color: Colour,
        user: UserExtended,
        mimimal_formatting: bool,
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
            minimal_formatting: mimimal_formatting,
        }
    }

    async fn handle_interactions(&mut self) -> Result<(), Error> {
        while let Some(interaction) = self
            .reply
            .message()
            .await?
            .id
            .collect_component_interactions(self.ctx.serenity_context())
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
            }
        }
        self.stop_paginator().await?;
        Ok(())
    }

    async fn update_page(&self, interaction: &ComponentInteraction) -> Result<(), Error> {
        let formatted_scores = format_score_list(
            &self.best_scores,
            None,
            Some(self.offset),
            self.minimal_formatting,
        )?;

        let footer = format!("Page {} of {}", self.page, self.max_pages);

        let user_link = format_user_link(i64::from(self.user.user_id));

        let embed = create_embed(OsuEmbedData {
            color: self.color,
            thumbnail: &self.user.avatar_url,
            description: &formatted_scores,
            footer: &footer,
            author_icon: &self.user.avatar_url,
            author_name: &self.user.username,
            author_url: &user_link,
            title: None,
            title_url: None,
        });

        let interaction_response = CreateInteractionResponseMessage::new().embed(embed);

        interaction
            .create_response(self.ctx.http(), UpdateMessage(interaction_response))
            .await?;

        Ok(())
    }

    async fn stop_paginator(&self) -> Result<(), Error> {
        let formatted_scores = format_score_list(
            &self.best_scores,
            None,
            Some(self.offset),
            self.minimal_formatting,
        )?;

        let footer = format!("Page {} of {}", self.page, self.max_pages);

        let user_link = format_user_link(i64::from(self.user.user_id));

        let embed = create_embed(OsuEmbedData {
            color: self.color,
            thumbnail: &self.user.avatar_url,
            description: &formatted_scores,
            footer: &footer,
            author_icon: &self.user.avatar_url,
            author_name: &self.user.username,
            author_url: &user_link,
            title: None,
            title_url: None,
        });

        let builder = CreateReply::default().embed(embed).components(vec![]);

        self.reply.edit(self.ctx, builder).await?;

        Ok(())
    }
}
