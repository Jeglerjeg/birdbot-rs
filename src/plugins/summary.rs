use crate::{Context, Error};
use markov::Chain;
use poise::futures_util::StreamExt;
use poise::serenity_prelude::{Message, UserId};
use tracing::log::{error, info};

pub struct SummaryMessage {
    pub content: String,
    pub discord_id: i64,
    pub is_bot: bool,
    pub author_id: i64,
}

impl From<Message> for SummaryMessage {
    fn from(discord_message: Message) -> SummaryMessage {
        SummaryMessage {
            content: discord_message.content,
            discord_id: i64::from(discord_message.id),
            is_bot: discord_message.author.bot,
            author_id: i64::from(discord_message.author.id),
        }
    }
}

pub async fn download_messages(ctx: &Context<'_>) -> Result<Vec<SummaryMessage>, Error> {
    let mut downloaded_messages: Vec<SummaryMessage> = Vec::new();
    let mut message_iterator = ctx.channel_id().messages_iter(&ctx).boxed();
    while let Some(message) = message_iterator.next().await {
        match message {
            Ok(message) => downloaded_messages.push(message.into()),
            Err(error) => error!("{error}"),
        }
    }
    Ok(downloaded_messages)
}

pub fn filter_messages(
    messages: Vec<SummaryMessage>,
    include_bots: Option<bool>,
    users: Vec<UserId>,
) -> Vec<String> {
    let mut filtered_messages: Vec<String> = Vec::new();
    let include_bots = include_bots.unwrap_or(false);
    for message in messages {
        if message.content.is_empty() {
            continue;
        }
        if !include_bots && message.is_bot {
            continue;
        }
        if !users.is_empty() && !users.iter().any(|x| x.0.get() == message.author_id as u64) {
            continue;
        }
        filtered_messages.push(message.content);
    }
    filtered_messages
}

pub fn generate_message(chain: Chain<String>) -> Option<String> {
    let mut generated_string = chain.generate_str();
    let mut tries = 0;
    while generated_string.chars().count() > 2000 {
        if tries == 10000 {
            return None;
        }
        tries += 1;
        info!("Regenerating string");
        generated_string = chain.generate_str();
    }
    Some(generated_string)
}

#[poise::command(prefix_command, slash_command)]
pub async fn summary(
    ctx: Context<'_>,
    include_bots: Option<bool>,
    users: Vec<UserId>,
) -> Result<(), Error> {
    let downloaded_messages = download_messages(&ctx).await?;
    let filtered_messages = filter_messages(downloaded_messages, include_bots, users);

    if filtered_messages.is_empty() {
        ctx.say("No messages matching filters.").await?;
    } else {
        let mut chain = Chain::new();
        chain.feed(filtered_messages);
        let generated_message = generate_message(chain);
        if let Some(message) = generated_message {
            ctx.say(message).await?;
        } else {
            ctx.say("Unable to generate a response.").await?;
        }
    }

    Ok(())
}
