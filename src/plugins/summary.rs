use crate::models::summary_messages::NewDbSummaryMessage;
use crate::utils::db::summary_messages;
use crate::{Context, Error};
use diesel_async::AsyncPgConnection;
use markov::Chain;
use poise::futures_util::StreamExt;
use poise::serenity_prelude::{ChannelId, Message, UserId};
use tracing::log::{error, info};

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
            Ok(message) => downloaded_messages.push(message.into()),
            Err(error) => error!("{error}"),
        }
    }
    if !downloaded_messages.is_empty() {
        summary_messages::create(connection, &downloaded_messages).await?;
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
        users.into_iter().map(|x| i64::from(x)).collect(),
        channel_ids.into_iter().map(|x| i64::from(x)).collect(),
    )
    .await?;
    Ok(messages)
}

pub fn generate_message(chain: Chain<String>) -> Option<String> {
    let mut generated_string = chain.generate_str();
    info!("{}", generated_string.chars().count());
    let mut tries = 0;
    while generated_string.chars().count() > 2000 {
        if tries == 1000 {
            return None;
        }
        info!("{}", generated_string.chars().count());
        tries += 1;
        generated_string = chain.generate_str();
    }
    Some(generated_string)
}
#[poise::command(prefix_command, slash_command, owners_only)]
pub async fn summary_enable(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let mut connection = ctx.data().db_pool.get().await?;
    ctx.say("Downloading messages, this may take a while.")
        .await?;
    download_messages(&ctx, &mut connection).await?;
    let downloaded_messages =
        summary_messages::count_entries(&mut connection, i64::from(ctx.channel_id())).await?;
    ctx.say(format!("Downloaded {} messages", downloaded_messages))
        .await?;
    Ok(())
}

#[poise::command(slash_command)]
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
        for message_string in filtered_messages {
            chain.feed_str(&message_string);
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
