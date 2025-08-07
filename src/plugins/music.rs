use crate::utils::misc::get_guild_channel;
use crate::{Context, Data, Error};
use aformat::aformat;
use dashmap::DashMap;
use poise::serenity_prelude::model::colour::colours::roles::BLUE;
use poise::serenity_prelude::{
    ChannelId, CreateEmbed, CreateMessage, GenericChannelId, GuildId, Http, User, async_trait,
};
use songbird::input::{AuxMetadata, Compose, YoutubeDl};
use songbird::tracks::{PlayMode, Track};
use songbird::{Event, EventContext, EventHandler as VoiceEventHandler, Songbird, TrackEvent};
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tracing::{error, info};

pub struct PlayingGuilds {
    pub guilds: DashMap<GuildId, Guild>,
}

pub struct Guild {
    queued_tracks: Queue,
    volume: f32,
}

pub struct Queue {
    pub queue: HashMap<u128, QueuedTrack>,
}

pub struct QueuedTrack {
    pub requested: User,
    pub skipped: Vec<u64>,
    pub metadata: AuxMetadata,
}

static PLAYING_GUILDS: OnceLock<PlayingGuilds> = OnceLock::new();

static MAX_SONGS_QUEUED: OnceLock<u16> = OnceLock::new();

static MAX_MUSIC_DURATION: OnceLock<Duration> = OnceLock::new();

fn get_http_client(ctx: Context<'_>) -> reqwest::Client {
    ctx.data().http_client.clone()
}

fn format_duration(duration: &Duration, play_time: Option<Duration>) -> String {
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
        duration = format!("Duration: {}", format_duration(&length, play_time));
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
    channel_id: GenericChannelId,
    http: &Arc<Http>,
    metadata: &AuxMetadata,
    action: &str,
    play_time: Option<Duration>,
) -> Result<(), Error> {
    let thumbnail_url = match &metadata.thumbnail {
        Some(thumbnail) => thumbnail,
        _ => "",
    };

    let embed = CreateEmbed::new()
        .description(format!("{}\n{}", action, format_track(metadata, play_time)))
        .color(BLUE)
        .thumbnail(thumbnail_url);

    let builder = CreateMessage::default().embed(embed);

    channel_id.send_message(http, builder).await?;

    Ok(())
}

pub async fn check_for_empty_channel(
    ctx: &poise::serenity_prelude::Context,
    guild: Option<GuildId>,
) -> Result<(), Error> {
    let Some(guild_id) = guild else {
        return Ok(());
    };

    let manager = get_manager(ctx);

    let guild_handler = manager.get(guild_id);

    if let Some(guild_handler) = guild_handler {
        let lock = guild_handler.lock().await;
        let channel = lock.current_channel();
        drop(lock);
        if channel.is_none() {
            return Ok(());
        }
        let channel_id = ChannelId::from(
            channel
                .ok_or("Failed to parse channel ID in check_for_empty_channel")?
                .get(),
        );
        let channel = get_guild_channel(&ctx.http, &ctx.cache, channel_id, guild_id)
            .await?
            .ok_or("Failed to get guild channel in check_for_empty")?;
        if channel.members(&ctx.cache)?.len() <= 1 {
            leave(ctx, Some(guild_id)).await?;
        }
    }

    Ok(())
}

pub async fn leave(
    ctx: &poise::serenity_prelude::Context,
    guild: Option<GuildId>,
) -> Result<(), Error> {
    let Some(guild_id) = guild else {
        return Ok(());
    };

    let manager = get_manager(ctx);

    if let Some(guild_handler) = manager.get(guild_id) {
        let lock = guild_handler.lock().await;
        let channel = lock.current_channel();
        drop(lock);
        if channel.is_none() {
            return Ok(());
        }
        manager.remove(guild_id).await?;
    }

    let playing_guilds = PLAYING_GUILDS.get_or_init(|| PlayingGuilds {
        guilds: DashMap::new(),
    });

    if playing_guilds.guilds.get(&guild_id).is_some() {
        playing_guilds.guilds.remove(&guild_id);
    }

    Ok(())
}

#[inline]
pub fn get_manager(ctx: &poise::serenity_prelude::Context) -> Arc<Songbird> {
    ctx.data::<Data>().songbird.clone()
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

struct TrackStartNotifier {
    guild_id: GuildId,
    channel_id: GenericChannelId,
    http: Arc<Http>,
}

#[async_trait]
impl VoiceEventHandler for TrackStartNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx
            && let Some(track) = track_list.first()
        {
            let Some(playing_guild) = PLAYING_GUILDS
                .get_or_init(|| PlayingGuilds {
                    guilds: DashMap::new(),
                })
                .guilds
                .get(&self.guild_id)
            else {
                error!("Failed to get playing guild in voice handler");
                return None;
            };
            let queued_track = playing_guild
                .queued_tracks
                .queue
                .get(&track.1.uuid().as_u128());
            if let Some(queued_track) = queued_track {
                let metadata = queued_track.metadata.clone();
                drop(playing_guild);
                if let Err(why) =
                    send_track_embed(self.channel_id, &self.http, &metadata, "Now playing:", None)
                        .await
                {
                    error!("Failed to send track starting message: {}", why);
                }
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
        if let EventContext::Track(track_list) = ctx {
            let manager = get_manager(&self.ctx);

            if let Some(handler) = manager.get(self.guild_id) {
                let handler_lock = handler.lock().await;
                if handler_lock.queue().is_empty() {
                    drop(handler_lock);
                    if let Err(why) = leave(&self.ctx, Option::from(self.guild_id)).await {
                        error!("Failed to leave voice channel: {}", why);
                    }
                } else {
                    drop(handler_lock);
                    let Some(mut playing_guild) = PLAYING_GUILDS
                        .get_or_init(|| PlayingGuilds {
                            guilds: DashMap::new(),
                        })
                        .guilds
                        .get_mut(&self.guild_id)
                    else {
                        error!("Failed to get playing guild in voice handler");
                        return None;
                    };

                    for track in *track_list {
                        playing_guild
                            .queued_tracks
                            .queue
                            .remove(&track.1.uuid().as_u128());
                    }
                }
            }
        }
        None
    }
}

async fn join(ctx: Context<'_>) -> Result<bool, Error> {
    let guild = ctx
        .guild()
        .ok_or("Failed to get guild in join function")?
        .clone();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|voice_state| voice_state.channel_id);

    let Some(connect_to) = channel_id else {
        ctx.say("Not in a voice channel").await?;
        return Ok(false);
    };

    let manager = get_manager(ctx.serenity_context());

    if let Ok(handler) = manager.join(guild_id, connect_to).await {
        let mut handler_lock = handler.lock().await;

        handler_lock.add_global_event(
            Event::Track(TrackEvent::End),
            TrackEndNotifier {
                guild_id,
                ctx: ctx.serenity_context().clone(),
            },
        );

        handler_lock.add_global_event(
            Event::Track(TrackEvent::Playable),
            TrackStartNotifier {
                guild_id,
                channel_id: ctx.channel_id(),
                http: ctx.serenity_context().http.clone(),
            },
        );

        handler_lock.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
        drop(handler_lock);

        PLAYING_GUILDS
            .get_or_init(|| PlayingGuilds {
                guilds: DashMap::new(),
            })
            .guilds
            .insert(
                guild_id,
                Guild {
                    queued_tracks: Queue {
                        queue: HashMap::new(),
                    },
                    volume: 0.6,
                },
            );
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

async fn queue(
    ctx: Context<'_>,
    mut url: String,
    guild_id: GuildId,
    first_queue: bool,
) -> Result<(), Error> {
    let http_client = get_http_client(ctx);

    if !url.starts_with("http") {
        url = format!("ytsearch1:{url}");
    }

    let mut source = YoutubeDl::new(http_client, url);

    let metadata = source.aux_metadata().await?;

    let manager = get_manager(ctx.serenity_context());

    let Some(handler) = manager.get(guild_id) else {
        return Ok(());
    };

    if let Some(duration) = metadata.duration {
        let max_duration = MAX_MUSIC_DURATION.get_or_init(|| {
            Duration::from_secs(
                env::var("MAX_MUSIC_DURATION")
                    .unwrap_or_else(|_| String::from("600"))
                    .parse::<u64>()
                    .expect("Failed to parse max music duration.")
                    * 60,
            )
        });
        if &duration > max_duration {
            let empty = handler.lock().await.queue().is_empty();
            ctx.say(format!(
                "Song is longer than the max allowed duration of {}",
                format_duration(max_duration, None)
            ))
            .await?;
            if empty {
                leave(ctx.serenity_context(), ctx.guild_id()).await?;
            }
            return Ok(());
        }
    }

    let mut handler_lock = handler.lock().await;

    let Some(guild_id) = ctx.guild_id() else {
        drop(handler_lock);
        error!("Failed to get guild ID in queue function");
        ctx.say("Something went wrong, please try again later.")
            .await?;
        return Ok(());
    };

    let Some(mut playing_guild) = PLAYING_GUILDS
        .get_or_init(|| PlayingGuilds {
            guilds: DashMap::new(),
        })
        .guilds
        .get_mut(&guild_id)
    else {
        drop(handler_lock);
        error!("Failed to get playing guild in queue function");
        ctx.say("Something went wrong, please try again later.")
            .await?;
        return Ok(());
    };

    let mut requested: u16 = 0;
    if !handler_lock.queue().is_empty() {
        for requester in &playing_guild.queued_tracks.queue {
            if requester.1.requested.id == ctx.author().id {
                requested += 1;
            }
        }
        let max_queued = MAX_SONGS_QUEUED.get_or_init(|| {
            env::var("MAX_SONGS_QUEUED")
                .unwrap_or_else(|_| String::from("6"))
                .parse::<u16>()
                .expect("Failed to parse max queued songs.")
        });
        if &requested >= max_queued {
            drop(playing_guild);
            ctx.say(
                aformat!(
                    "You have queued more than the maximum of {} songs.",
                    max_queued.to_arraystring()
                )
                .as_str(),
            )
            .await?;
            return Ok(());
        }
    }

    let mut track = Track::from(source);

    track = track.volume(playing_guild.volume);

    let track = handler_lock.enqueue(track).await;

    drop(handler_lock);

    let queued_track = QueuedTrack {
        metadata: metadata.clone(),
        requested: ctx.author().clone(),
        skipped: Vec::new(),
    };

    playing_guild
        .queued_tracks
        .queue
        .insert(track.uuid().as_u128(), queued_track);

    drop(playing_guild);

    if !first_queue {
        send_track_embed(
            ctx.channel_id(),
            &ctx.serenity_context().http,
            &metadata,
            "Queued:",
            None,
        )
        .await?;
    }

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
    let guild = ctx
        .guild()
        .ok_or("Failed to get guild in play function")?
        .clone();
    let guild_id = guild.id;

    let manager = get_manager(ctx.serenity_context());

    if manager.get(guild_id).is_some() {
        match queue(ctx, url_or_name, guild_id, false).await {
            Ok(()) => {}
            Err(why) => {
                ctx.say(format!("Couldn't queue item: {why}")).await?;
            }
        }
    } else {
        if !join(ctx).await? {
            return Ok(());
        }

        if manager.get(guild_id).is_some() {
            match queue(ctx, url_or_name, guild_id, true).await {
                Ok(()) => {}
                Err(why) => {
                    ctx.say(format!("Couldn't queue item: {why}")).await?;
                }
            }
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
    let guild = ctx
        .guild()
        .ok_or("Failed to get guild in skip function")?
        .clone();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|voice_state| voice_state.channel_id);

    let Some(user_channel) = channel_id else {
        ctx.say("Not in a voice channel").await?;
        return Ok(());
    };

    let manager = get_manager(ctx.serenity_context());

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;

        let channel_id = match handler.current_channel() {
            None => {
                drop(handler);
                error!("Failed to get current playing channel in skip function");
                ctx.say("Something went wrong, please try again later.")
                    .await?;
                return Ok(());
            }
            Some(channel_id) => ChannelId::from(channel_id.get()),
        };
        if user_channel.get() != channel_id.get() {
            drop(handler);
            ctx.say("Not connected to the voice channel").await?;
            return Ok(());
        }

        let queue = handler.queue();

        let Some(track) = queue.current() else {
            drop(handler);
            error!("Failed to get current track in skip function");
            ctx.say("Something went wrong, please try again later.")
                .await?;
            return Ok(());
        };

        let Some(mut playing_guild) = PLAYING_GUILDS
            .get_or_init(|| PlayingGuilds {
                guilds: DashMap::new(),
            })
            .guilds
            .get_mut(&guild_id)
        else {
            drop(handler);
            error!("Failed to get playing guilds in skip function");
            ctx.say("Something went wrong, please try again later.")
                .await?;
            return Ok(());
        };

        let Some(queued_track) = playing_guild
            .queued_tracks
            .queue
            .get_mut(&track.uuid().as_u128())
        else {
            drop(handler);
            drop(playing_guild);
            ctx.say("Something went wrong while skipping the track.")
                .await?;
            return Ok(());
        };

        if queued_track.requested.id == ctx.author().id {
            drop(queue.skip());
            let metadata = queued_track.metadata.clone();
            drop(handler);
            drop(playing_guild);
            send_track_embed(
                ctx.channel_id(),
                &ctx.serenity_context().http,
                &metadata,
                "Skipped:",
                None,
            )
            .await?;
        } else {
            if queued_track.skipped.contains(&ctx.author().id.get()) {
                drop(handler);
                drop(playing_guild);
                ctx.say("You've already skipped this track").await?;
                return Ok(());
            }

            let guild_channels = guild.id.channels(ctx.http()).await?;

            let needed_to_skip = match guild_channels.get(&channel_id) {
                None => {
                    drop(handler);
                    drop(playing_guild);
                    ctx.say("Something went wrong while skipping the track.")
                        .await?;
                    return Ok(());
                }
                Some(channel) => channel.members(ctx.cache())?.len() - 2,
            };

            queued_track.skipped.push(ctx.author().id.get());

            if queued_track.skipped.len() >= needed_to_skip {
                drop(queue.skip());
                let metadata = queued_track.metadata.clone();
                drop(handler);
                drop(playing_guild);
                send_track_embed(
                    ctx.channel_id(),
                    &ctx.serenity_context().http,
                    &metadata,
                    "Skipped:",
                    None,
                )
                .await?;
            } else {
                let skipped = queued_track.skipped.len();
                drop(handler);
                drop(playing_guild);
                ctx.say(
                    aformat!(
                        "Voted to skip the current song. `{}/{}`",
                        skipped.to_arraystring(),
                        needed_to_skip
                    )
                    .as_str(),
                )
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
    let guild = ctx
        .guild()
        .ok_or("Failed to get guild in undo function")?
        .clone();
    let guild_id = guild.id;

    let manager = get_manager(ctx.serenity_context());

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();

        if queue.is_empty() {
            drop(handler);
            ctx.say("No items queued").await?;
        } else {
            let Some(removed_item) = queue.dequeue(queue.len() - 1) else {
                drop(handler);
                error!("Failed to deque track in undo function");
                ctx.say("Something went wrong, please try again later.")
                    .await?;
                return Ok(());
            };
            let Some(playing_guild) = PLAYING_GUILDS
                .get_or_init(|| PlayingGuilds {
                    guilds: DashMap::new(),
                })
                .guilds
                .get(&guild_id)
            else {
                drop(handler);
                error!("Failed to get playing guild in undo function");
                ctx.say("Something went wrong, please try again later.")
                    .await?;
                return Ok(());
            };

            let Some(queued_track) = playing_guild
                .queued_tracks
                .queue
                .get(&removed_item.uuid().as_u128())
            else {
                drop(handler);
                drop(playing_guild);
                error!("Failed to skip track in undo function");
                ctx.say("Something went wrong while skipping the track.")
                    .await?;
                return Ok(());
            };

            let metadata = queued_track.metadata.clone();

            drop(handler);
            drop(playing_guild);
            send_track_embed(
                ctx.channel_id(),
                &ctx.serenity_context().http,
                &metadata,
                "Undid:",
                None,
            )
            .await?;
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
    let guild = ctx
        .guild()
        .ok_or("Failed to get guild in volume function.")?
        .clone();
    let guild_id = guild.id;

    let manager = get_manager(ctx.serenity_context());

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
        let Some(mut playing_guild) = PLAYING_GUILDS
            .get_or_init(|| PlayingGuilds {
                guilds: DashMap::new(),
            })
            .guilds
            .get_mut(&guild_id)
        else {
            drop(handler_lock);
            error!("Failed to get playing guild in volume function");
            ctx.say("Something went wrong while skipping the track.")
                .await?;
            return Ok(());
        };
        playing_guild.volume = adjusted_volume;

        let queue = handler_lock.queue();
        if queue.is_empty() {
            drop(handler_lock);
            drop(playing_guild);
            ctx.say("No items queued").await?;
        } else {
            for track in &queue.current_queue() {
                track.set_volume(adjusted_volume)?;
            }
            drop(handler_lock);
            drop(playing_guild);
            ctx.say(aformat!("Changed volume to {}%.", volume.to_arraystring()).as_str())
                .await?;
        }
    } else {
        let queue = handler_lock.queue();
        if let Some(track) = queue.current() {
            drop(handler_lock);
            ctx.say(format!(
                "Current volume is {}%.",
                (track.get_info().await?.volume * 100.0) as u32
            ))
            .await?;
        } else {
            drop(handler_lock);
            ctx.say("No items queued").await?;
        }
    }

    Ok(())
}

///Pause the currently playing song.
#[poise::command(prefix_command, slash_command, category = "Music", guild_only = true)]
pub async fn pause(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx
        .guild()
        .ok_or("Failed to get guild in pause function.")?
        .clone();
    let guild_id = guild.id;

    let manager = get_manager(ctx.serenity_context());

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
    let guild = ctx
        .guild()
        .ok_or("Failed to get guild in resume function.")?
        .clone();
    let guild_id = guild.id;

    let manager = get_manager(ctx.serenity_context());

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
    let guild = ctx
        .guild()
        .ok_or("Failed to get guild in now_playing function.")?
        .clone();
    let guild_id = guild.id;

    let manager = get_manager(ctx.serenity_context());

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if let Some(track) = queue.current() {
            drop(handler);

            let playing_guilds = PLAYING_GUILDS
                .get_or_init(|| PlayingGuilds {
                    guilds: DashMap::new(),
                })
                .guilds
                .get(&guild_id)
                .ok_or("Failed to get playing guild in now_playing function.")?;

            let Some(queued_track) = playing_guilds
                .queued_tracks
                .queue
                .get(&track.uuid().as_u128())
            else {
                drop(playing_guilds);
                ctx.say("Something went wrong while skipping the track.")
                    .await?;
                return Ok(());
            };

            let metadata = queued_track.metadata.clone();

            drop(playing_guilds);

            send_track_embed(
                ctx.channel_id(),
                &ctx.serenity_context().http,
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
