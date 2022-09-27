use crate::{Context, Error};
use lazy_static::lazy_static;
use poise::serenity_prelude;
use serenity::model::id::{ChannelId, GuildId};
use serenity::utils::colours::roles::BLUE;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use uuid::Uuid;

use songbird::input::Input;
use songbird::tracks::PlayMode;
use songbird::{
    input::Restartable, tracks::TrackHandle, Call, Event, EventContext,
    EventHandler as VoiceEventHandler, TrackEvent,
};
use tracing::error;

pub struct PlayingGuilds {
    pub guilds: HashMap<GuildId, Arc<Mutex<Requesters>>>,
}

pub struct Requesters {
    pub requester: HashMap<Uuid, Arc<Mutex<QueuedTrack>>>,
}

pub struct QueuedTrack {
    pub track: TrackHandle,
    pub requested: serenity_prelude::User,
    pub skipped: i8,
}

lazy_static! {
    static ref PLAYING_GUILDS: Arc<Mutex<PlayingGuilds>> = Arc::from(Mutex::from(PlayingGuilds {
        guilds: HashMap::new(),
    }));
}

lazy_static! {
    static ref MAX_SONGS_QUEUED: u16 = env::var("MAX_SONGS_QUEUED")
        .unwrap_or_else(|_| String::from("6"))
        .parse::<u16>()
        .expect("Failed to parse max queued songs.");
}

lazy_static! {
    static ref MAX_MUSIC_DURATION: Duration = Duration::from_secs(
        env::var("MAX_MUSIC_DURATION")
            .unwrap_or_else(|_| String::from("600"))
            .parse::<u64>()
            .expect("Failed to parse max music duration.")
            * 60
    );
}

fn format_duration(duration: Duration, play_time: Option<Duration>) -> String {
    if duration.as_secs() >= 3600 {
        match play_time {
            Some(play_time) => {
                let played_hours_minutes_and_seconds = (
                    (play_time.as_secs() / 60 / 60) % 60,
                    (play_time.as_secs() / 60) % 60,
                    play_time.as_secs() % 60,
                );

                let hours_minutes_and_seconds = (
                    (duration.as_secs() / 60 / 60) % 60,
                    (duration.as_secs() / 60) % 60,
                    duration.as_secs() % 60,
                );
                format!(
                    "**{:02}:{:02}:{:02}/{:02}:{:02}:{:02}**\n",
                    played_hours_minutes_and_seconds.0,
                    played_hours_minutes_and_seconds.1,
                    played_hours_minutes_and_seconds.2,
                    hours_minutes_and_seconds.0,
                    hours_minutes_and_seconds.1,
                    hours_minutes_and_seconds.2
                )
            }
            _ => {
                let hours_minutes_and_seconds = (
                    (duration.as_secs() / 60 / 60) % 60,
                    (duration.as_secs() / 60) % 60,
                    duration.as_secs() % 60,
                );
                format!(
                    "**{:02}:{:02}:{:02}**\n",
                    hours_minutes_and_seconds.0,
                    hours_minutes_and_seconds.1,
                    hours_minutes_and_seconds.2
                )
            }
        }
    } else {
        match play_time {
            Some(play_time) => {
                let played_minutes_and_seconds =
                    ((play_time.as_secs() / 60) % 60, play_time.as_secs() % 60);
                let minutes_and_seconds = ((duration.as_secs() / 60) % 60, duration.as_secs() % 60);
                format!(
                    "**{}:{:02}/{}:{:02}**\n",
                    played_minutes_and_seconds.0,
                    played_minutes_and_seconds.1,
                    minutes_and_seconds.0,
                    minutes_and_seconds.1
                )
            }
            _ => {
                let minutes_and_seconds = ((duration.as_secs() / 60) % 60, duration.as_secs() % 60);
                format!(
                    "**{}:{:02}**\n",
                    minutes_and_seconds.0, minutes_and_seconds.1
                )
            }
        }
    }
}

fn format_track(track: &TrackHandle, play_time: Option<Duration>) -> String {
    let title = match track.metadata().title.clone() {
        Some(title) => format!("**{}**\n", title),
        _ => String::from(""),
    };

    let duration: String;
    if let Some(length) = track.metadata().duration {
        duration = format!("Duration: {}", format_duration(length, play_time));
    } else {
        duration = "".into()
    }

    let url = match track.metadata().source_url.clone() {
        Some(url) => format!("**URL**: <{}>", url),
        _ => String::from(""),
    };

    format!("{}{}{}", title, duration, url)
}

async fn send_track_embed(
    ctx: Context<'_>,
    track: &TrackHandle,
    action: String,
    play_time: Option<Duration>,
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
            e.description(format!("{}\n{}", action, format_track(track, play_time)))
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
            leave(ctx, Some(guild_id)).await;
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

    let mut guild_lock = PLAYING_GUILDS.lock().await;
    if let Some(..) = guild_lock.guilds.get(&guild_id) {
        guild_lock.guilds.remove(&guild_id);
    };
    drop(guild_lock);
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
                } else {
                    let playing_guilds_lock = PLAYING_GUILDS.lock().await;
                    let mut guild_lock = playing_guilds_lock
                        .guilds
                        .get(&self.guild_id)
                        .unwrap()
                        .lock()
                        .await;
                    for track in _track_list.iter() {
                        guild_lock.requester.remove(&(*track).1.uuid());
                    }
                    drop(guild_lock);
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
    let mut guild_lock = PLAYING_GUILDS.lock().await;
    guild_lock.guilds.insert(
        guild_id,
        Arc::from(Mutex::from(Requesters {
            requester: HashMap::new(),
        })),
    );
    drop(guild_lock);

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
    subcommands("play", "skip", "undo", "volume", "resume", "pause", "now_playing"),
    aliases("m"),
    guild_only = true
)]
pub(crate) async fn music(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Use one of the subcommands to use the music bot!")
        .await?;

    Ok(())
}

async fn queue(
    ctx: Context<'_>,
    handler_lock: Arc<Mutex<Call>>,
    requesters: Arc<Mutex<Requesters>>,
    url: String,
) {
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

    let input = Input::from(source);

    let mut handler = handler_lock.lock().await;
    let requester_lock = requesters.lock().await;

    let mut requested: u16 = 0;
    if !handler.queue().is_empty() {
        for requester in &requester_lock.requester {
            if requester.1.lock().await.requested.id == ctx.author().id {
                requested += 1;
            }
        }
        if requested >= *MAX_SONGS_QUEUED {
            ctx.say(format!(
                "You have queued more than the maximum of {} songs.",
                *MAX_SONGS_QUEUED
            ))
            .await
            .expect("Couldn't send message");
            drop(handler);
            drop(requester_lock);
            return;
        }
    }
    drop(requester_lock);

    if let Some(duration) = input.metadata.duration {
        if duration > *MAX_MUSIC_DURATION {
            ctx.say(format!(
                "Song is longer than the max allowed duration of {}",
                format_duration(*MAX_MUSIC_DURATION, None)
            ))
            .await
            .expect("Failed to send message");
            if handler.queue().is_empty() {
                drop(handler);
                leave(ctx.discord().clone(), ctx.guild_id()).await;
            }
            return;
        }
    }

    let track = handler.enqueue_source(input);

    drop(handler);

    track.set_volume(0.6).expect("Failed to queue track");

    let queued_track = QueuedTrack {
        track: track.clone(),
        requested: ctx.author().clone(),
        skipped: 0,
    };
    let mut requester_lock = requesters.lock().await;

    requester_lock
        .requester
        .insert(track.uuid(), Arc::from(Mutex::from(queued_track)));

    drop(requester_lock);

    send_track_embed(ctx, &track, String::from("Queued:"), None)
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
    ctx.defer().await?;
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
    if let Some(handler_lock) = manager.get(guild_id) {
        let guild_lock = PLAYING_GUILDS.lock().await;

        let requesters = guild_lock
            .guilds
            .get(&ctx.guild_id().unwrap())
            .unwrap()
            .clone();
        drop(guild_lock);
        queue(ctx, handler_lock, requesters, url_or_name).await;
    } else {
        if !join(ctx).await {
            return Ok(());
        }

        let guild_lock = PLAYING_GUILDS.lock().await;
        let requesters = guild_lock
            .guilds
            .get(&ctx.guild_id().unwrap())
            .unwrap()
            .clone();
        drop(guild_lock);

        if let Some(handler_lock) = manager.get(guild_id) {
            queue(ctx, handler_lock, requesters.clone(), url_or_name).await;
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

    let channel_id = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|voice_state| voice_state.channel_id);

    let user_channel = match channel_id {
        Some(channel) => channel,
        None => {
            ctx.say("Not in a voice channel").await?;
            return Ok(());
        }
    };

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue().clone();

        let track = queue.current().unwrap();
        let guild_data_lock = PLAYING_GUILDS.lock().await;
        let playing_guild = guild_data_lock.guilds.get(&guild_id).unwrap();
        let guild_lock = playing_guild.lock().await;
        let queued_track = guild_lock.requester.get(&track.uuid()).unwrap();
        let mut queue_lock = queued_track.lock().await;
        if queue_lock.requested.id == ctx.author().id {
            let _ = queue.skip();
            drop(handler);
            send_track_embed(ctx, &track, String::from("Skipped:"), None).await?;
        } else {
            let channel_id = handler.current_channel().unwrap();
            if user_channel.0 != channel_id.0 {
                ctx.say("Not connected to the voice channel").await?;
                return Ok(());
            }
            let guild_channels = guild.channels(ctx.discord()).await.unwrap();
            let channel = guild_channels.get(&ChannelId::from(channel_id.0)).unwrap();
            let needed_to_skip = (channel.members(ctx.discord()).await.unwrap().len() - 2) as i8;
            queue_lock.skipped += 1;
            if queue_lock.skipped >= needed_to_skip {
                let _ = queue.skip();
                drop(handler);
                send_track_embed(ctx, &track, String::from("Skipped:"), None).await?;
            } else {
                ctx.say(format!(
                    "Voted to skip the current song. `{}/{}`",
                    queue_lock.skipped, needed_to_skip
                ))
                .await?;
            }
        }
        drop(queue_lock);
        drop(guild_lock);
        drop(guild_data_lock);
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
        let queue = handler.queue().clone();
        drop(handler);
        if !queue.is_empty() {
            let removed_item = queue.dequeue(queue.len() - 1).unwrap();
            send_track_embed(ctx, &removed_item.handle(), String::from("Undid:"), None).await?;
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
        let queue = handler.queue().clone();
        drop(handler);
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
        let queue = handler.queue().clone();
        drop(handler);
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
        let queue = handler.queue().clone();
        drop(handler);
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

#[poise::command(
    prefix_command,
    slash_command,
    category = "Music",
    guild_only = true,
    aliases("np", "playing")
)]
pub async fn now_playing(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue().clone();
        drop(handler);
        match queue.current() {
            Some(track) => {
                send_track_embed(
                    ctx,
                    &track,
                    String::from("Now playing:"),
                    Some(track.get_info().await.unwrap().play_time),
                )
                .await?;
            }
            _ => {
                ctx.say("No item playing.").await?;
            }
        }
    } else {
        ctx.say("Not in a voice channel.").await?;
    }
    Ok(())
}
