use crate::{Context, Error};
use poise::serenity_prelude;
use serenity::model::id::{ChannelId, GuildId};
use serenity::utils::colours::roles::BLUE;
use std::sync::Arc;
use tokio::sync::Mutex;

use songbird::tracks::PlayMode;
use songbird::{
    input::Restartable, tracks::TrackHandle, Call, Event, EventContext,
    EventHandler as VoiceEventHandler, TrackEvent,
};
use tracing::error;

fn format_track(track: &TrackHandle) -> String {
    let title = match track.metadata().title.clone() {
        Some(title) => format!("**{}**\n", title),
        _ => String::from(""),
    };

    let duration = match track.metadata().duration {
        Some(length) => {
            let minutes_and_seconds = ((length.as_secs() / 60) % 60, length.as_secs() % 60);
            format!(
                "Duration: **{}:{:02}**\n",
                minutes_and_seconds.0, minutes_and_seconds.1
            )
        }
        _ => String::from(""),
    };

    let url = match track.metadata().source_url.clone() {
        Some(url) => format!("**URL**: <{}>", url),
        _ => String::from(""),
    };

    return format!("{}{}{}", title, duration, url);
}

async fn send_track_embed(
    ctx: Context<'_>,
    track: &TrackHandle,
    action: String,
) -> Result<(), Error> {
    let color = ctx
        .author_member()
        .await
        .unwrap()
        .colour(ctx.discord())
        .unwrap_or(BLUE);

    let thumbnail_url = match track.metadata().thumbnail.clone() {
        Some(thumbnail) => thumbnail,
        _ => String::from(""),
    };

    ctx.send(|m| {
        m.embed(|e| {
            e.description(format!("{}\n{}", action, format_track(track)))
                .color(color)
                .thumbnail(thumbnail_url)
        })
    })
    .await?;
    Ok(())
}

pub async fn check_for_empty_channel(ctx: serenity_prelude::Context, guild: Option<GuildId>) {
    let guild_id = match guild {
        Some(guild) => guild,
        _ => {
            return;
        }
    };

    let manager = songbird::get(&ctx.clone())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let guild_handler = manager.get(guild_id);

    if let Some(guild_handler) = guild_handler {
        let lock = guild_handler.lock().await;
        let channel = lock.current_channel();
        drop(lock);
        if channel.is_none() {
            return;
        }
        let channel_id = ChannelId::from(channel.unwrap().0);
        let guild = ctx.http.get_guild(guild_id.0).await.unwrap();
        let guild_channels = guild.channels(&ctx).await.unwrap();
        let channel = guild_channels.get(&channel_id).unwrap();
        if channel.members(&ctx).await.unwrap().len() <= 1 {
            leave(ctx, Option::from(guild_id)).await;
        }
    }
}

pub async fn leave(ctx: serenity_prelude::Context, guild: Option<GuildId>) {
    let guild_id = match guild {
        Some(guild) => guild,
        _ => {
            return;
        }
    };

    let manager = songbird::get(&ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(guild_handler) = manager.get(guild_id) {
        let lock = guild_handler.lock().await;
        let channel = lock.current_channel();
        drop(lock);
        if channel.is_none() {
            return;
        }
        manager
            .remove(guild_id)
            .await
            .expect("Failed to leave channel.");
    };
}

struct TrackEndNotifier {
    guild_id: GuildId,
    ctx: serenity_prelude::Context,
}

#[poise::async_trait]
impl VoiceEventHandler for TrackEndNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(_track_list) = ctx {
            let manager = songbird::get(&self.ctx)
                .await
                .expect("Songbird Voice client placed in at initialisation.")
                .clone();
            if let Some(handler_lock) = manager.get(self.guild_id) {
                let handler = handler_lock.lock().await;
                if handler.queue().is_empty() {
                    drop(handler);
                    leave(self.ctx.clone(), Option::from(self.guild_id)).await;
                }
            }
        }
        None
    }
}

async fn join(ctx: Context<'_>) -> bool {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            ctx.say("Not in a voice channel")
                .await
                .expect("Failed to send message");
            return false;
        }
    };

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let (handle_lock, _success) = manager.join(guild_id, connect_to).await;
    let mut handle = handle_lock.lock().await;

    let leave_context = ctx.discord().clone();
    handle.add_global_event(
        Event::Track(TrackEvent::End),
        TrackEndNotifier {
            guild_id,
            ctx: leave_context,
        },
    );

    true
}

#[poise::command(
    prefix_command,
    slash_command,
    category = "Music",
    subcommands("play", "skip", "undo", "volume", "resume", "pause"),
    aliases("m"),
    guild_only = true
)]
pub(crate) async fn music(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Use one of the subcommands to use the music bot!")
        .await?;

    Ok(())
}

async fn queue(ctx: Context<'_>, handler_lock: Arc<Mutex<Call>>, url: String) {
    // Here, we use lazy restartable sources to make sure that we don't pay
    // for decoding, playback on tracks which aren't actually live yet.
    let source = if !url.starts_with("http") {
        match Restartable::ytdl_search(url, true).await {
            Ok(source) => source,
            Err(why) => {
                error!("Err starting source: {:?}", why);

                ctx.say("Error sourcing ffmpeg")
                    .await
                    .expect("Failed to send message");

                return;
            }
        }
    } else {
        match Restartable::ytdl(url, true).await {
            Ok(source) => source,
            Err(why) => {
                error!("Err starting source: {:?}", why);

                ctx.say("Error sourcing ffmpeg")
                    .await
                    .expect("Failed to send message");

                return;
            }
        }
    };

    let mut handler = handler_lock.lock().await;

    let track = handler.enqueue_source(source.into());
    track.set_volume(0.6).expect("Failed to queue track");

    send_track_embed(ctx, &track, String::from("Queued:"))
        .await
        .expect("Couldn't send track embed.");
}

#[poise::command(
    prefix_command,
    slash_command,
    aliases("p"),
    category = "Music",
    guild_only = true
)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "Name or URL of song to play"] url_or_name: String,
) -> Result<(), Error> {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        queue(ctx, handler_lock, url_or_name).await;
    } else {
        if !join(ctx).await {
            return Ok(());
        }
        if let Some(handler_lock) = manager.get(guild_id) {
            queue(ctx, handler_lock, url_or_name).await;
        } else {
            ctx.say("Couldn't join voice channel").await?;
        }
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    aliases("s"),
    category = "Music",
    guild_only = true
)]
pub async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue().clone();
        let _ = queue.skip();
        drop(handler);

        let track = queue.current().unwrap();

        send_track_embed(ctx, &track, String::from("Skipped:")).await?;
    } else {
        ctx.say("Not in a voice channel to play in").await?;
    }

    Ok(())
}

#[poise::command(prefix_command, slash_command, category = "Music", guild_only = true)]
pub async fn undo(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if !queue.is_empty() {
            let removed_item = queue.dequeue(queue.len() - 1).unwrap();
            send_track_embed(ctx, &removed_item.handle(), String::from("Undid:")).await?;
        } else {
            ctx.say("No items queued").await?;
        }
    } else {
        ctx.say("Not in a voice channel to play in").await?;
    }

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    category = "Music",
    aliases("vol"),
    guild_only = true
)]
pub async fn volume(
    ctx: Context<'_>,
    #[description = "Volume to change the track to, accepts 1-200"] mut new_volume: u32,
) -> Result<(), Error> {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;
    if new_volume > 200 {
        new_volume = 200;
    }
    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        match queue.current() {
            Some(track) => {
                track.set_volume(new_volume as f32 / 100.0)?;
                ctx.say(format!("Changed volume to {}%.", new_volume))
                    .await?;
            }
            _ => {
                ctx.say("No items queued").await?;
            }
        }
    } else {
        ctx.say("Not in a voice channel.").await?;
    }

    Ok(())
}

#[poise::command(prefix_command, slash_command, category = "Music", guild_only = true)]
pub async fn pause(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        match queue.current() {
            Some(track) => {
                if track.get_info().await.unwrap().playing != PlayMode::Play {
                    ctx.say("Current track isn't playing.").await?;
                    return Ok(());
                }
                if let Err(e) = track.pause() {
                    ctx.say(format!("Failed: {:?}", e)).await?;
                    return Ok(());
                }
                ctx.say("Paused song").await?;
            }
            _ => {
                ctx.say("No items queued").await?;
            }
        }
    } else {
        ctx.say("Not in a voice channel.").await?;
    }

    Ok(())
}

#[poise::command(prefix_command, slash_command, category = "Music", guild_only = true)]
pub async fn resume(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        match queue.current() {
            Some(track) => {
                if track.get_info().await.unwrap().playing != PlayMode::Pause {
                    ctx.say("Current track isn't paused.").await?;
                    return Ok(());
                }
                if let Err(e) = track.play() {
                    ctx.say(format!("Failed: {:?}", e)).await?;
                    return Ok(());
                }
                ctx.say("Resumed song").await?;
            }
            _ => {
                ctx.say("No items queued").await?;
            }
        }
    } else {
        ctx.say("Not in a voice channel.").await?;
    }

    Ok(())
}
