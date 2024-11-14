use crate::{Context, Error};
use poise::serenity_prelude::{
    Cache, ChannelId, GuildChannel, GuildId, Http, Mentionable, Message,
};
use poise::PrefixContext;
use std::fmt::Write;

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

pub fn content_safe(message: &Message, cache: &Cache) -> String {
    let mut result = message.content.to_string();

    // First replace all user mentions.
    for u in &message.mentions {
        let mut at_distinct = String::with_capacity(38);
        at_distinct.push('@');
        at_distinct.push_str(&u.name);
        if let Some(discriminator) = u.discriminator {
            at_distinct.push('#');
            write!(at_distinct, "{:04}", discriminator.get()).unwrap();
        }

        let mut m = u.mention().to_string();
        // Check whether we're replacing a nickname mention or a normal mention.
        // `UserId::mention` returns a normal mention. If it isn't present in the message, it's
        // a nickname mention.
        if !result.contains(&m) {
            m.insert(2, '!');
        }

        result = result.replace(&m, &at_distinct);
    }

    // Then replace all role mentions.
    if let Some(guild_id) = message.guild_id {
        for id in &message.mention_roles {
            let mention = id.mention().to_string();

            if let Some(guild) = cache.guild(guild_id) {
                if let Some(role) = guild.roles.get(id) {
                    result = result.replace(&mention, &format!("@{}", role.name));
                    continue;
                }
            }

            result = result.replace(&mention, "@deleted-role");
        }
    }

    // And finally replace everyone and here mentions.
    result
        .replace("@everyone", "@\u{200B}everyone")
        .replace("@here", "@\u{200B}here")
}
