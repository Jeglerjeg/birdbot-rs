use crate::models::summary_enabled_guilds::NewSummaryEnabledGuild;
use crate::models::summary_messages::NewDbSummaryMessage;
use crate::utils::db::{summary_enabled_guilds, summary_messages};
use crate::{Context, Data, Error};
use dashmap::DashMap;

use diesel_async::AsyncPgConnection;
use markov::Chain;

use poise::futures_util::StreamExt;
use poise::serenity_prelude::{ChannelId, Message, UserId};
use std::sync::OnceLock;
use tracing::log::error;

pub struct SummaryEnabledGuilds {
    pub guilds: DashMap<i64, Vec<i64>>,
}

impl SummaryEnabledGuilds {
    fn new() -> SummaryEnabledGuilds {
        SummaryEnabledGuilds {
            guilds: DashMap::new(),
        }
    }
}

static SUMMARY_ENABLED_GUILDS: OnceLock<SummaryEnabledGuilds> = OnceLock::new();

impl From<Message> for NewDbSummaryMessage {
    fn from(discord_message: Message) -> NewDbSummaryMessage {
        NewDbSummaryMessage {
            content: discord_message.content,
            discord_id: i64::from(discord_message.id),
            is_bot: discord_message.author.bot,
            author_id: i64::from(discord_message.author.id),
            channel_id: i64::from(discord_message.channel_id),
        }
    }
}

pub async fn download_messages(
    ctx: &Context<'_>,
    connection: &mut AsyncPgConnection,
) -> Result<(), Error> {
    let mut downloaded_messages: Vec<NewDbSummaryMessage> = Vec::new();
    let mut message_iterator = ctx.channel_id().messages_iter(&ctx).boxed();
    while let Some(message) = message_iterator.next().await {
        if downloaded_messages.len() == 1000 {
            summary_messages::create(connection, &downloaded_messages).await?;
            downloaded_messages.clear();
        }
        match message {
            Ok(message) => {
                if message.content.is_empty() {
                    continue;
                }
                downloaded_messages.push(message.into());
            }
            Err(error) => error!("{error}"),
        }
    }
    if !downloaded_messages.is_empty() {
        summary_messages::create(connection, &downloaded_messages).await?;
    }
    Ok(())
}

pub async fn add_message(message: &Message, data: &Data) -> Result<(), Error> {
    let Some(guild_id) = message.guild_id else {
        return Ok(());
    };
    if message.content.is_empty() {
        return Ok(());
    }

    let enabled_guilds = SUMMARY_ENABLED_GUILDS.get_or_init(SummaryEnabledGuilds::new);

    match enabled_guilds.guilds.get(&i64::from(guild_id)) {
        None => {
            enabled_guilds.guilds.insert(i64::from(guild_id), {
                let connection = &mut data.db_pool.get().await?;
                match summary_enabled_guilds::read(connection, i64::from(guild_id)).await {
                    Ok(guild) => guild
                        .channel_ids
                        .iter()
                        .flatten()
                        .copied()
                        .collect::<Vec<i64>>(),
                    Err(_) => Vec::new(),
                }
            });
        }
        Some(summary_enabled_guild) => {
            if summary_enabled_guild
                .value()
                .contains(&i64::from(message.channel_id))
            {
                drop(summary_enabled_guild);
                let connection = &mut data.db_pool.get().await?;
                summary_messages::create(
                    connection,
                    &vec![NewDbSummaryMessage::from(message.clone())],
                )
                .await?;
            } else {
                drop(summary_enabled_guild);
            }
        }
    }

    Ok(())
}

pub async fn get_filtered_messages(
    connection: &mut AsyncPgConnection,
    include_bots: bool,
    phrase: Option<String>,
    users: Vec<UserId>,
    channel_ids: Vec<ChannelId>,
) -> Result<Vec<Vec<String>>, Error> {
    let messages = summary_messages::read(
        connection,
        include_bots,
        phrase,
        users.into_iter().map(i64::from).collect(),
        channel_ids.into_iter().map(i64::from).collect(),
    )
    .await?;
    Ok(messages)
}

pub fn generate_message(chain: Chain<String>) -> Option<String> {
    let mut generated_string = chain.generate().join(" ");
    let mut tries = 0;
    while generated_string.chars().count() > 2000 {
        if tries == 1000 {
            return None;
        }
        tries += 1;
        generated_string = chain.generate().join(" ");
    }
    drop(chain);
    Some(generated_string)
}

#[poise::command(prefix_command, owners_only, guild_only)]
pub async fn summary_disable(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let guild_id = ctx
        .guild_id()
        .ok_or("Failed to get guild id in summary_enable")?;
    let mut connection = ctx.data().db_pool.get().await?;
    let summary_enabled_guild =
        summary_enabled_guilds::read(&mut connection, i64::from(guild_id)).await;
    if let Ok(mut summary_enabled_guild) = summary_enabled_guild {
        let channels = summary_enabled_guild
            .channel_ids
            .iter()
            .flatten()
            .collect::<Vec<_>>();
        if channels.contains(&&i64::from(ctx.channel_id())) {
            summary_enabled_guild
                .channel_ids
                .retain(|element| element != &Some(i64::from(ctx.channel_id())));
            let new_guild = NewSummaryEnabledGuild {
                guild_id: summary_enabled_guild.guild_id,
                channel_ids: summary_enabled_guild.channel_ids,
            };
            summary_enabled_guilds::delete_channel(
                &mut connection,
                i64::from(guild_id),
                &new_guild,
            )
            .await?;
            summary_messages::delete(&mut connection, i64::from(ctx.channel_id())).await?;
        }
    }
    ctx.say("Disabled summaries in this channel").await?;
    Ok(())
}

#[poise::command(prefix_command, owners_only, guild_only)]
pub async fn summary_enable(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let guild_id = ctx
        .guild_id()
        .ok_or("Failed to get guild id in summary_enable")?;
    let mut connection = ctx.data().db_pool.get().await?;
    let enabled_guild = summary_enabled_guilds::read(&mut connection, i64::from(guild_id)).await;

    let enabled_guilds = SUMMARY_ENABLED_GUILDS.get_or_init(SummaryEnabledGuilds::new);

    if let Ok(mut guild) = enabled_guild {
        guild.channel_ids.push(Some(i64::from(ctx.channel_id())));
        let new_guild = NewSummaryEnabledGuild {
            guild_id: guild.id,
            channel_ids: guild.channel_ids,
        };
        summary_enabled_guilds::update(&mut connection, guild.id, &new_guild).await?;

        enabled_guilds
            .guilds
            .entry(i64::from(guild_id))
            .or_default()
            .push(i64::from(ctx.channel_id()));
    } else {
        let new_guild = NewSummaryEnabledGuild {
            guild_id: i64::from(guild_id),
            channel_ids: vec![Some(i64::from(ctx.channel_id()))],
        };
        let inserted_guild = summary_enabled_guilds::create(&mut connection, &new_guild).await?;

        enabled_guilds.guilds.insert(
            inserted_guild.guild_id,
            inserted_guild
                .channel_ids
                .iter()
                .flatten()
                .copied()
                .collect::<Vec<i64>>(),
        );
    }

    ctx.say("Downloading messages, this may take a while.")
        .await?;

    download_messages(&ctx, &mut connection).await?;
    let downloaded_messages =
        summary_messages::count_entries(&mut connection, i64::from(ctx.channel_id())).await?;

    ctx.say(format!("Downloaded {downloaded_messages} messages"))
        .await?;

    Ok(())
}

#[poise::command(slash_command, guild_only)]
pub async fn summary(
    ctx: Context<'_>,
    phrase: Option<String>,
    include_bots: Option<bool>,
    users: Vec<UserId>,
    #[min = 1]
    #[max = 10]
    n_grams: Option<usize>,
    mut channels: Vec<ChannelId>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let mut connection = ctx.data().db_pool.get().await?;
    if channels.is_empty() {
        channels.push(ctx.channel_id());
    }
    let filtered_messages = get_filtered_messages(
        &mut connection,
        include_bots.unwrap_or(false),
        phrase,
        users,
        channels,
    )
    .await?;

    if filtered_messages.is_empty() {
        ctx.say("No messages matching filters.").await?;
    } else {
        let mut chain = Chain::of_order(n_grams.unwrap_or(2));

        for sentence in filtered_messages {
            chain.feed(sentence);
        }
        let generated_message = generate_message(chain);
        if let Some(message) = generated_message {
            ctx.say(message).await?;
        } else {
            ctx.say("Unable to generate a response.").await?;
        }
    }

    Ok(())
}
