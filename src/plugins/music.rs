use crate::{Context, Error};
use lazy_static::lazy_static;
use poise::serenity_prelude::model::colour::colours::roles::BLUE;
use poise::serenity_prelude::{async_trait, ChannelId, CreateEmbed, GuildId, User};
use poise::CreateReply;
use songbird::input::{AuxMetadata, Compose, YoutubeDl};
use songbird::tracks::{PlayMode, Track};
use songbird::{
    tracks::TrackHandle, Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent,
};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, info};

pub struct PlayingGuilds {
    pub guilds: HashMap<GuildId, Arc<Mutex<Guild>>>,
}

pub struct Guild {
    queued_tracks: Queue,
    volume: f32,
}

pub struct Queue {
    pub queue: HashMap<u128, Mutex<QueuedTrack>>,
}

pub struct QueuedTrack {
    pub track: TrackHandle,
    pub requested: User,
    pub skipped: Vec<u64>,
    pub metadata: AuxMetadata,
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

fn get_http_client(ctx: Context<'_>) -> reqwest::Client {
    ctx.data().http_client.clone()
}

fn format_duration(duration: Duration, play_time: Option<Duration>) -> String {
    if duration.as_secs() >= 3600 {
        if let Some(play_time) = play_time {
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
        } else {
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
    } else if let Some(play_time) = play_time {
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
    } else {
        let minutes_and_seconds = ((duration.as_secs() / 60) % 60, duration.as_secs() % 60);
        format!(
            "**{}:{:02}**\n",
            minutes_and_seconds.0, minutes_and_seconds.1
        )
    }
}

fn format_track(metadata: &AuxMetadata, play_time: Option<Duration>) -> String {
    let title = match &metadata.title {
        Some(title) => format!("**{title}**\n"),
        _ => String::new(),
    };

    let duration: String;
    if let Some(length) = metadata.duration {
        duration = format!("Duration: {}", format_duration(length, play_time));
    } else {
        duration = String::new();
    }

    let url = match &metadata.source_url {
        Some(url) => format!("**URL**: <{url}>"),
        _ => String::new(),
    };

    format!("{title}{duration}{url}")
}

async fn send_track_embed(
    ctx: Context<'_>,
    metadata: &AuxMetadata,
    action: &str,
    play_time: Option<Duration>,
) -> Result<(), Error> {
    let color = ctx
        .author_member()
        .await
        .unwrap()
        .colour(ctx.discord())
        .unwrap_or(BLUE);

    let thumbnail_url = match &metadata.thumbnail {
        Some(thumbnail) => thumbnail,
        _ => "",
    };

    let embed = CreateEmbed::new()
        .description(format!("{}\n{}", action, format_track(metadata, play_time)))
        .color(color)
        .thumbnail(thumbnail_url);

    let builder = CreateReply::default().embed(embed);

    ctx.send(builder).await?;

    Ok(())
}

pub async fn check_for_empty_channel(
    ctx: &poise::serenity_prelude::Context,
    guild: Option<GuildId>,
) -> Result<(), Error> {
    let Some(guild_id) = guild else {
            return Ok(());
        };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let guild_handler = manager.get(guild_id);

    if let Some(guild_handler) = guild_handler {
        let lock = guild_handler.lock().await;
        let channel = lock.current_channel();
        drop(lock);
        if channel.is_none() {
            return Ok(());
        }
        let channel_id = ChannelId::from(channel.unwrap().0);
        let guild = ctx.http.get_guild(guild_id).await?;
        let guild_channels = guild.channels(&ctx).await?;
        let channel = guild_channels.get(&channel_id).unwrap();
        if channel.members(ctx)?.len() <= 1 {
            leave(ctx, Some(guild_id)).await?;
        }
    };

    Ok(())
}

pub async fn leave(
    ctx: &poise::serenity_prelude::Context,
    guild: Option<GuildId>,
) -> Result<(), Error> {
    let Some(guild_id) = guild else {
        return Ok(());
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(guild_handler) = manager.get(guild_id) {
        let lock = guild_handler.lock().await;
        let channel = lock.current_channel();
        drop(lock);
        if channel.is_none() {
            return Ok(());
        }
        manager.remove(guild_id).await?;
    };

    let mut guild_lock = PLAYING_GUILDS.lock().await;
    if guild_lock.guilds.get(&guild_id).is_some() {
        guild_lock.guilds.remove(&guild_id);
    };
    drop(guild_lock);

    Ok(())
}

struct TrackErrorNotifier;

#[async_trait]
impl VoiceEventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                info!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }

        None
    }
}

struct TrackEndNotifier {
    guild_id: GuildId,
    ctx: poise::serenity_prelude::Context,
}

#[async_trait]
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
                    if let Err(why) = leave(&self.ctx, Option::from(self.guild_id)).await {
                        error!("Failed to leave voice channel: {}", why);
                    };
                } else {
                    let playing_guilds_lock = PLAYING_GUILDS.lock().await;
                    let mut guild_lock = playing_guilds_lock
                        .guilds
                        .get(&self.guild_id)
                        .unwrap()
                        .lock()
                        .await;

                    for track in _track_list.iter() {
                        guild_lock
                            .queued_tracks
                            .queue
                            .remove(&(track).1.uuid().as_u128());
                    }
                    drop(guild_lock);
                    drop(playing_guilds_lock);
                }
            }
        }
        None
    }
}

async fn join(ctx: Context<'_>) -> Result<bool, Error> {
    let guild = ctx.guild().unwrap().clone();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|voice_state| voice_state.channel_id);

    let Some(connect_to) = channel_id else {
        ctx.say("Not in a voice channel").await?;
        return Ok(false);
    };

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Ok(handle_lock) = manager.join(guild_id, connect_to).await {
        let mut handle = handle_lock.lock().await;
        let mut guild_lock = PLAYING_GUILDS.lock().await;
        guild_lock.guilds.insert(
            guild_id,
            Arc::from(Mutex::from(Guild {
                queued_tracks: Queue {
                    queue: HashMap::new(),
                },
                volume: 0.6,
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

        handle.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
        drop(handle);
    }

    Ok(true)
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

async fn queue(ctx: Context<'_>, mut url: String, guild_id: GuildId) -> Result<(), Error> {
    // Here, we use lazy restartable sources to make sure that we don't pay
    // for decoding, playback on tracks which aren't actually live yet.

    let http_client = get_http_client(ctx);

    if !url.starts_with("http") {
        url = format!("ytsearch1:{url}");
    }

    let mut source = YoutubeDl::new(http_client, url);

    let metadata = match source.aux_metadata().await {
        Ok(metadata) => metadata,
        Err(why) => {
            error!("{}", why);
            return Ok(());
        }
    };

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let Some(handler) = manager.get(guild_id) else {
        return Ok(());
    };

    let mut handler_lock = handler.lock().await;

    let guild_lock = PLAYING_GUILDS.lock().await;

    let mut playing_guild = guild_lock
        .guilds
        .get(&ctx.guild_id().unwrap())
        .unwrap()
        .lock()
        .await;

    let mut requested: u16 = 0;
    if !handler_lock.queue().is_empty() {
        for requester in &playing_guild.queued_tracks.queue {
            let request_lock = requester.1.lock().await;
            if request_lock.requested.id == ctx.author().id {
                requested += 1;
            }
            drop(request_lock);
        }
        if requested >= *MAX_SONGS_QUEUED {
            drop(handler_lock);
            drop(playing_guild);
            ctx.say(format!(
                "You have queued more than the maximum of {} songs.",
                *MAX_SONGS_QUEUED
            ))
            .await?;
            return Ok(());
        }
    }

    if let Some(duration) = metadata.duration {
        if duration > *MAX_MUSIC_DURATION {
            let empty = handler_lock.queue().is_empty();
            drop(handler_lock);
            drop(playing_guild);
            ctx.say(format!(
                "Song is longer than the max allowed duration of {}",
                format_duration(*MAX_MUSIC_DURATION, None)
            ))
            .await?;
            if empty {
                leave(ctx.discord(), ctx.guild_id()).await?;
            }
            return Ok(());
        }
    }

    let mut track = Track::from(source);

    track = track.volume(playing_guild.volume);

    let track = handler_lock.enqueue(track).await;

    drop(handler_lock);

    let queued_track = QueuedTrack {
        track: track.clone(),
        metadata: metadata.clone(),
        requested: ctx.author().clone(),
        skipped: Vec::new(),
    };

    playing_guild
        .queued_tracks
        .queue
        .insert(track.uuid().as_u128(), Mutex::from(queued_track));

    drop(playing_guild);

    send_track_embed(ctx, &metadata, "Queued:", None).await?;

    Ok(())
}

///Play a song in a guild voice channel.
#[poise::command(
    prefix_command,
    slash_command,
    aliases("p"),
    category = "Music",
    guild_only = true
)]
pub async fn play(
    ctx: Context<'_>,
    #[rest]
    #[description = "Name or URL of song to play"]
    url_or_name: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let guild = ctx.guild().unwrap().clone();
    let guild_id = guild.id;

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if manager.get(guild_id).is_some() {
        queue(ctx, url_or_name, guild_id).await?;
    } else {
        if !join(ctx).await? {
            return Ok(());
        }

        if manager.get(guild_id).is_some() {
            queue(ctx, url_or_name, guild_id).await?;
        }
    }
    Ok(())
}

///Skip the song currently playing.
#[poise::command(
    prefix_command,
    slash_command,
    aliases("s"),
    category = "Music",
    guild_only = true
)]
pub async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx.guild().unwrap().clone();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|voice_state| voice_state.channel_id);

    let Some(user_channel) = channel_id else {
        ctx.say("Not in a voice channel").await?;
        return Ok(());
    };

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;

        let channel_id = handler.current_channel().unwrap();
        if user_channel.0 != channel_id.0 {
            ctx.say("Not connected to the voice channel").await?;
            return Ok(());
        }

        let queue = handler.queue();

        let track = queue.current().unwrap();
        let playing_guilds_lock = PLAYING_GUILDS.lock().await;
        let current_guild_lock = playing_guilds_lock
            .guilds
            .get(&guild_id)
            .unwrap()
            .lock()
            .await;

        let mut track_lock = current_guild_lock
            .queued_tracks
            .queue
            .get(&track.uuid().as_u128())
            .unwrap()
            .lock()
            .await;

        if track_lock.requested.id == ctx.author().id {
            drop(queue.skip());
            let metadata = track_lock.metadata.clone();
            drop(handler);
            drop(track_lock);
            drop(current_guild_lock);
            drop(playing_guilds_lock);
            send_track_embed(ctx, &metadata, "Skipped:", None).await?;
        } else {
            if track_lock.skipped.contains(&ctx.author().id.0.get()) {
                drop(handler);
                drop(track_lock);
                drop(current_guild_lock);
                drop(playing_guilds_lock);
                ctx.say("You've already skipped this track").await?;
                return Ok(());
            }

            let guild_channels = guild.channels(ctx.discord()).await?;
            let channel = guild_channels.get(&ChannelId::from(channel_id.0)).unwrap();
            let needed_to_skip = channel.members(ctx.discord())?.len() - 2;

            track_lock.skipped.push(ctx.author().id.0.get());

            if track_lock.skipped.len() >= needed_to_skip {
                drop(queue.skip());
                let metadata = track_lock.metadata.clone();
                drop(handler);
                drop(track_lock);
                drop(current_guild_lock);
                drop(playing_guilds_lock);
                send_track_embed(ctx, &metadata, "Skipped:", None).await?;
            } else {
                let skipped = track_lock.skipped.len();
                drop(handler);
                drop(track_lock);
                drop(current_guild_lock);
                drop(playing_guilds_lock);
                ctx.say(format!(
                    "Voted to skip the current song. `{skipped}/{needed_to_skip}`",
                ))
                .await?;
            }
        }
    } else {
        ctx.say("Not in a voice channel to play in").await?;
    }

    Ok(())
}

///Undo your previously queued song. This will not *skip* the song if it's playing.
#[poise::command(prefix_command, slash_command, category = "Music", guild_only = true)]
pub async fn undo(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx.guild().unwrap().clone();
    let guild_id = guild.id;

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();

        if queue.is_empty() {
            drop(handler);
            ctx.say("No items queued").await?;
        } else {
            let removed_item = queue.dequeue(queue.len() - 1).unwrap();
            let playing_guilds_lock = PLAYING_GUILDS.lock().await;
            let current_guild_lock = playing_guilds_lock
                .guilds
                .get(&guild_id)
                .unwrap()
                .lock()
                .await;

            let track_lock = current_guild_lock
                .queued_tracks
                .queue
                .get(&removed_item.uuid().as_u128())
                .unwrap()
                .lock()
                .await;

            let metadata = track_lock.metadata.clone();

            drop(handler);
            drop(track_lock);
            drop(current_guild_lock);
            drop(playing_guilds_lock);
            send_track_embed(ctx, &metadata, "Undid:", None).await?;
        }
    } else {
        ctx.say("Not in a voice channel to play in").await?;
    }

    Ok(())
}

///Set the volume of the player. Volume should be a number from 1-200.
#[poise::command(
    prefix_command,
    slash_command,
    category = "Music",
    aliases("vol", "v"),
    guild_only = true
)]
pub async fn volume(
    ctx: Context<'_>,
    #[min = 0]
    #[max = 200]
    #[description = "Volume to change the track to, accepts 1-200"]
    new_volume: Option<u8>,
) -> Result<(), Error> {
    let guild = ctx.guild().unwrap().clone();
    let guild_id = guild.id;

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let Some(handler) = manager.get(guild_id) else {
        ctx.say("Not in a voice channel.").await?;
        return Ok(());
    };

    let handler_lock = handler.lock().await;
    if let Some(mut volume) = new_volume {
        if volume > 200 {
            volume = 200;
        }

        let adjusted_volume = f32::from(volume) / 100.0;
        let guild_lock = PLAYING_GUILDS.lock().await;
        let mut playing_guild_lock = guild_lock.guilds.get(&guild_id).unwrap().lock().await;
        playing_guild_lock.volume = adjusted_volume;
        drop(playing_guild_lock);
        drop(guild_lock);

        let queue = handler_lock.queue();
        if queue.is_empty() {
            ctx.say("No items queued").await?;
        } else {
            for track in &queue.current_queue() {
                track.set_volume(adjusted_volume)?;
            }
            ctx.say(format!("Changed volume to {volume}%.")).await?;
        }
    } else {
        let queue = handler_lock.queue();
        match queue.current() {
            Some(track) => {
                ctx.say(format!(
                    "Current volume is {}%.",
                    (track.get_info().await?.volume * 100.0) as u32
                ))
                .await?;
            }
            _ => {
                ctx.say("No items queued").await?;
            }
        }
    }
    drop(handler_lock);

    Ok(())
}

///Pause the currently playing song.
#[poise::command(prefix_command, slash_command, category = "Music", guild_only = true)]
pub async fn pause(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx.guild().unwrap().clone();
    let guild_id = guild.id;

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if let Some(track) = queue.current() {
            drop(handler);
            if track.get_info().await?.playing != PlayMode::Play {
                ctx.say("Current track isn't playing.").await?;
                return Ok(());
            }
            if let Err(why) = track.pause() {
                ctx.say(format!("Failed: {why:?}")).await?;
                return Ok(());
            }
            ctx.say("Paused song").await?;
        } else {
            drop(handler);
            ctx.say("No items queued").await?;
        }
    } else {
        ctx.say("Not in a voice channel.").await?;
    }

    Ok(())
}

///Resume the currently paused song.
#[poise::command(prefix_command, slash_command, category = "Music", guild_only = true)]
pub async fn resume(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx.guild().unwrap().clone();
    let guild_id = guild.id;

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if let Some(track) = queue.current() {
            drop(handler);
            if track.get_info().await?.playing != PlayMode::Pause {
                ctx.say("Current track isn't paused.").await?;
                return Ok(());
            }
            if let Err(why) = track.play() {
                ctx.say(format!("Failed: {why:?}")).await?;
                return Ok(());
            }
            ctx.say("Resumed song").await?;
        } else {
            drop(handler);
            ctx.say("No items queued").await?;
        }
    } else {
        ctx.say("Not in a voice channel.").await?;
    }

    Ok(())
}

///Display the currently playing song.
#[poise::command(
    prefix_command,
    slash_command,
    category = "Music",
    guild_only = true,
    aliases("np", "playing")
)]
pub async fn now_playing(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx.guild().unwrap().clone();
    let guild_id = guild.id;

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if let Some(track) = queue.current() {
            drop(handler);
            let playing_guilds_lock = PLAYING_GUILDS.lock().await;
            let current_guild_lock = playing_guilds_lock
                .guilds
                .get(&guild_id)
                .unwrap()
                .lock()
                .await;

            let track_lock = current_guild_lock
                .queued_tracks
                .queue
                .get(&track.uuid().as_u128())
                .unwrap()
                .lock()
                .await;

            let metadata = track_lock.metadata.clone();

            drop(track_lock);
            drop(current_guild_lock);
            drop(playing_guilds_lock);

            send_track_embed(
                ctx,
                &metadata,
                "Now playing:",
                Some(track.get_info().await?.play_time),
            )
            .await?;
        } else {
            drop(handler);
            ctx.say("No item playing.").await?;
        }
    } else {
        ctx.say("Not in a voice channel.").await?;
    }
    Ok(())
}
