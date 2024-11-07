use crate::{Context, Error};
use poise::PrefixContext;
use serenity::all::Message;

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
