use crate::{Context, Error};
use poise::serenity_prelude::{Cache, ChannelId, GuildChannel, GuildId, Http, Message};
use poise::PrefixContext;

pub fn remove_trailing_zeros(number: f64, precision: usize) -> Result<f64, Error> {
    Ok((format!("{number:.precision$}").parse::<f64>()? * 100_000_000.0).round() / 100_000_000.0)
}

pub fn get_reply(ctx: Context<'_>) -> Option<Message> {
    let mut reply: Option<Message> = None;
    if let Context::Prefix(PrefixContext { msg, .. }) = ctx {
        if let Some(msg_reply) = &msg.referenced_message {
            reply = Some(*msg_reply.clone());
        }
    }
    reply
}

pub async fn get_guild_channel(
    http: &Http,
    cache: &Cache,
    channel_id: ChannelId,
    guild_id: GuildId,
) -> Result<Option<GuildChannel>, Error> {
    if let Some(cached_guild) = cache.guild(guild_id) {
        return Ok(cached_guild.channels.get(&channel_id).cloned());
    }

    let guild_channels = guild_id.channels(http).await?;
    Ok(guild_channels.get(&channel_id).cloned())
}
