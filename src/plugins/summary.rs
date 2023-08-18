use crate::models::summary_enabled_guilds::NewSummaryEnabledGuild;
use crate::models::summary_messages::NewDbSummaryMessage;
use crate::utils::db::{summary_enabled_guilds, summary_messages};
use crate::{Context, Data, Error};
use dashmap::DashMap;
use diesel_async::AsyncPgConnection;
use lazy_static::lazy_static;
use markov::Chain;
use poise::futures_util::StreamExt;
use poise::serenity_prelude::{ChannelId, Message, UserId};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use tracing::log::error;

pub struct SummaryEnabledGuilds {
    pub guilds: DashMap<i64, Vec<i64>>,
}

lazy_static! {
    static ref SUMMARY_ENABLED_GUILDS: SummaryEnabledGuilds = SummaryEnabledGuilds {
        guilds: DashMap::new(),
    };
}

impl From<Message> for NewDbSummaryMessage {
    fn from(discord_message: Message) -> NewDbSummaryMessage {
        NewDbSummaryMessage {
            content: discord_message.content,
            guild_id: i64::from(discord_message.guild_id.unwrap()),
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
            Ok(mut message) => {
                if message.content.is_empty() {
                    continue;
                }
                message.guild_id = ctx.guild_id();
                downloaded_messages.push(message.into())
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

    let summary_enabled_guild = SUMMARY_ENABLED_GUILDS
        .guilds
        .entry(i64::from(guild_id))
        .or_insert({
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

    if summary_enabled_guild
        .value()
        .contains(&i64::from(message.channel_id))
    {
        let connection = &mut data.db_pool.get().await?;
        summary_messages::create(
            connection,
            &vec![NewDbSummaryMessage::from(message.clone())],
        )
        .await?;
    }

    Ok(())
}

pub async fn get_filtered_messages(
    connection: &mut AsyncPgConnection,
    include_bots: bool,
    phrase: Option<String>,
    users: Vec<UserId>,
    channel_ids: Vec<ChannelId>,
) -> Result<Vec<String>, Error> {
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
    Some(generated_string)
}

#[poise::command(prefix_command, owners_only, guild_only)]
pub async fn summary_disable(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let guild_id = ctx
        .guild_id()
        .ok_or("Failed to get guild id in summary_enable")?;
    let mut connection = ctx.data().db_pool.get().await?;
    if summary_enabled_guilds::read(&mut connection, i64::from(guild_id))
        .await
        .is_ok()
    {
        summary_enabled_guilds::delete(&mut connection, i64::from(guild_id)).await?;
        summary_messages::delete(&mut connection, i64::from(guild_id)).await?;
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

    match enabled_guild {
        Ok(mut guild) => {
            guild.channel_ids.push(Some(i64::from(ctx.channel_id())));
            let new_guild = NewSummaryEnabledGuild {
                guild_id: guild.id,
                channel_ids: guild.channel_ids,
            };
            summary_enabled_guilds::update(&mut connection, guild.id, &new_guild).await?;

            SUMMARY_ENABLED_GUILDS
                .guilds
                .entry(i64::from(guild_id))
                .or_default()
                .push(i64::from(ctx.channel_id()));
        }
        Err(_) => {
            let new_guild = NewSummaryEnabledGuild {
                guild_id: i64::from(guild_id),
                channel_ids: vec![Some(i64::from(ctx.channel_id()))],
            };
            let inserted_guild =
                summary_enabled_guilds::create(&mut connection, &new_guild).await?;

            SUMMARY_ENABLED_GUILDS.guilds.insert(
                inserted_guild.guild_id,
                inserted_guild
                    .channel_ids
                    .iter()
                    .flatten()
                    .copied()
                    .collect::<Vec<i64>>(),
            );
        }
    }

    ctx.say("Downloading messages, this may take a while.")
        .await?;

    download_messages(&ctx, &mut connection).await?;
    let downloaded_messages =
        summary_messages::count_entries(&mut connection, i64::from(ctx.channel_id())).await?;

    ctx.say(format!("Downloaded {} messages", downloaded_messages))
        .await?;

    Ok(())
}

#[poise::command(slash_command, guild_only)]
pub async fn summary(
    ctx: Context<'_>,
    phrase: Option<String>,
    include_bots: Option<bool>,
    users: Vec<UserId>,
    mut channels: Vec<ChannelId>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let mut connection = ctx.data().db_pool.get().await?;
    if channels.is_empty() {
        channels.push(ctx.channel_id())
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
        let mut chain = Chain::new();
        for value in filtered_messages
            .into_par_iter()
            .map(|message_string| {
                message_string
                    .split_whitespace()
                    .filter(|word| !word.is_empty())
                    .map(|s| s.to_owned())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
        {
            chain.feed(value);
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
