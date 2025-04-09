use crate::models::beatmaps::Beatmap;
use crate::models::beatmapsets::Beatmapset;
use crate::models::linked_osu_profiles::LinkedOsuProfile;
use crate::models::osu_files::OsuFile;
use crate::models::osu_notifications::NewOsuNotification;
use crate::models::osu_users::OsuUser;
use crate::utils::db::{linked_osu_profiles, osu_guild_channels, osu_notifications, osu_users};
use crate::utils::osu::caching::{get_beatmap, get_updated_beatmapset};
use crate::utils::osu::calculate::calculate;
use crate::utils::osu::embeds::create_embed;
use crate::utils::osu::map_format::format_beatmapset;
use crate::utils::osu::misc::{
    add_profile_data, calculate_potential_acc, gamemode_from_string, get_osu_user, is_playing,
};
use crate::utils::osu::misc_format::{format_beatmap_link, format_footer, format_user_link};
use crate::utils::osu::regex::get_beatmap_info;
use crate::utils::osu::score_format::format_new_score;
use crate::{Error, Pool};
use chrono::Utc;
use dashmap::DashMap;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use poise::serenity_prelude::model::colour::colours::roles::BLUE;
use poise::serenity_prelude::{
    Cache, CacheHttp, ChannelId, CreateEmbed, CreateMessage, Http, UserId,
};
use rosu_v2::Osu;
use rosu_v2::model::GameMode;
use rosu_v2::prelude::{EventBeatmap, EventType, RankStatus};
use std::env;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info};

static UPDATE_INTERVAL: OnceLock<u64> = OnceLock::new();

static NOT_PLAYING_SKIP: OnceLock<i32> = OnceLock::new();

pub static SCORE_NOTIFICATIONS: OnceLock<DashMap<i64, Vec<u64>>> = OnceLock::new();

pub struct OsuTracker {
    pub cache: Arc<Cache>,
    pub http: Arc<Http>,
    pub osu_client: Arc<Osu>,
    pub pool: Pool<AsyncDieselConnectionManager<AsyncPgConnection>>,
}
impl OsuTracker {
    pub async fn tracking_loop(&mut self) -> Result<(), Error> {
        let mut interval = tokio::time::interval(Duration::from_secs(
            UPDATE_INTERVAL
                .get_or_init(|| {
                    env::var("UPDATE_INTERVAL")
                        .unwrap_or_else(|_| String::from("30"))
                        .parse::<u64>()
                        .expect("Failed to parse tracking update interval.")
                })
                .to_owned(),
        ));
        loop {
            interval.tick().await;
            let connection = &mut match self.pool.get().await {
                Ok(connection) => connection,
                Err(why) => {
                    error!("Failed to connect to database {}", why);
                    continue;
                }
            };
            let profiles = match linked_osu_profiles::get_all(connection).await {
                Ok(profiles) => profiles,
                Err(why) => {
                    error!("Failed to get linked osu profiles {}", why);
                    continue;
                }
            };
            for profile in profiles {
                if let Err(why) = self.update_user_data(&profile, connection).await {
                    error!("Error occurred while running tracking loop: {}", why);
                }
            }
        }
    }

    async fn update_user_data(
        &mut self,
        linked_profile: &LinkedOsuProfile,
        connection: &mut AsyncPgConnection,
    ) -> Result<(), Error> {
        let user = match get_osu_user(
            &self.cache,
            UserId::from(u64::try_from(linked_profile.id)?),
            u64::try_from(linked_profile.home_guild)?,
        )? {
            Some(user) => user.clone(),
            _ => return Ok(()),
        };

        if let Ok(mut profile) = osu_users::read(connection, linked_profile.osu_id).await {
            profile.ticks += 1;

            let not_playing_skip = NOT_PLAYING_SKIP
                .get_or_init(|| {
                    env::var("NOT_PLAYING_SKIP")
                        .unwrap_or_else(|_| String::from("10"))
                        .parse::<i32>()
                        .expect("Failed to parse tracking not playing skip.")
                })
                .to_owned();

            if profile.ticks > not_playing_skip {
                profile.ticks = 0;
                osu_users::update_ticks(connection, profile.id, profile.ticks).await?;
                return Ok(());
            }

            if is_playing(&self.cache, user.id, linked_profile.home_guild)?
                || (profile.ticks.eq(&not_playing_skip))
            {
                osu_users::update_ticks(connection, profile.id, profile.ticks).await?;

                if let Err(why) = self
                    .notify_recent(&profile, connection, linked_profile)
                    .await
                {
                    error!("Error occurred while running tracking loop: {}", why);
                    return Ok(());
                }
            } else {
                osu_users::update_ticks(connection, profile.id, profile.ticks).await?;
            }
        } else {
            add_profile_data(
                self.osu_client.clone(),
                u32::try_from(linked_profile.osu_id)?,
                gamemode_from_string(&linked_profile.mode)
                    .ok_or("Failed to parse gamemode in update_user_data function")?,
                connection,
            )
            .await?;
        }

        Ok(())
    }

    async fn get_notify_beatmapset(
        &mut self,
        connection: &mut AsyncPgConnection,
        beatmapset_url: &str,
    ) -> Result<(Beatmapset, Vec<(Beatmap, OsuFile)>), Error> {
        let beatmapset_info = get_beatmap_info(&format!("https://osu.ppy.sh{beatmapset_url}"))?;

        let beatmapset_id = beatmapset_info
            .beatmapset_id
            .ok_or("Failed to get beatmapset ID in notify_beatmap_update")?;

        sleep(Duration::from_secs(45)).await;

        get_updated_beatmapset(connection, self.osu_client.clone(), beatmapset_id as u32).await
    }

    async fn notify_recent(
        &mut self,
        new: &OsuUser,
        connection: &mut AsyncPgConnection,
        linked_profile: &LinkedOsuProfile,
    ) -> Result<(), Error> {
        let last_notifications =
            if let Ok(updates) = osu_notifications::read(connection, linked_profile.osu_id).await {
                updates
            } else {
                let item = NewOsuNotification {
                    id: linked_profile.osu_id,
                    last_pp: Utc::now(),
                    last_event: Utc::now(),
                };
                osu_notifications::create(connection, &item).await?
            };

        let mut recent_events = self
            .osu_client
            .recent_activity(u32::try_from(new.id)?)
            .await?;
        recent_events.reverse();

        let mut notified = false;

        for event in &recent_events {
            match &event.event_type {
                EventType::Rank {
                    grade: _grade,
                    rank,
                    mode,
                    beatmap,
                    user: _user,
                } => {
                    if last_notifications.last_event.timestamp() > event.created_at.unix_timestamp()
                    {
                        continue;
                    }

                    if rank > &50 {
                        continue;
                    }

                    self.notify_leaderboard_score(beatmap, mode, new, connection, linked_profile)
                        .await?;

                    notified = true;
                }
                EventType::BeatmapsetApprove {
                    approval,
                    beatmapset,
                    user: _user,
                } => {
                    if last_notifications.last_event.timestamp() > event.created_at.unix_timestamp()
                    {
                        continue;
                    }

                    let beatmapset = self
                        .get_notify_beatmapset(connection, &beatmapset.url)
                        .await?;

                    let mut status = format!(
                        "[**{} - {}**]({}) by [**{}**]({}) ",
                        beatmapset.0.artist,
                        beatmapset.0.title,
                        format_beatmap_link(None, beatmapset.0.id, None),
                        new.username,
                        format_user_link(new.id),
                    );

                    match approval {
                        RankStatus::Ranked => status.push_str("has been ranked!"),
                        RankStatus::Approved => status.push_str("has been ranked!"),
                        RankStatus::Qualified => status.push_str("has been qualified!"),
                        RankStatus::Loved => status.push_str("has been loved!"),
                        _ => {}
                    }

                    self.notify_beatmap_update(beatmapset, connection, linked_profile, &status)
                        .await?;

                    notified = true;
                }
                EventType::BeatmapsetRevive {
                    beatmapset,
                    user: _user,
                } => {
                    if last_notifications.last_event.timestamp() > event.created_at.unix_timestamp()
                    {
                        continue;
                    }

                    let beatmapset = self
                        .get_notify_beatmapset(connection, &beatmapset.url)
                        .await?;

                    let status = format!(
                        "[**{} - {}**]({}) has been revived from eternal slumber by [**{}**]({})",
                        beatmapset.0.artist,
                        beatmapset.0.title,
                        format_beatmap_link(None, beatmapset.0.id, None),
                        new.username,
                        format_user_link(new.id),
                    );

                    self.notify_beatmap_update(beatmapset, connection, linked_profile, &status)
                        .await?;

                    notified = true;
                }
                EventType::BeatmapsetUpdate {
                    beatmapset,
                    user: _user,
                } => {
                    if last_notifications.last_event.timestamp() > event.created_at.unix_timestamp()
                    {
                        continue;
                    }

                    let beatmapset = self
                        .get_notify_beatmapset(connection, &beatmapset.url)
                        .await?;

                    let status = format!(
                        "[**{}**]({}) has updated the beatmap [**{} - {}**]({})",
                        new.username,
                        format_user_link(new.id),
                        beatmapset.0.artist,
                        beatmapset.0.title,
                        format_beatmap_link(None, beatmapset.0.id, None)
                    );

                    self.notify_beatmap_update(beatmapset, connection, linked_profile, &status)
                        .await?;

                    notified = true;
                }
                EventType::BeatmapsetUpload {
                    beatmapset,
                    user: _user,
                } => {
                    if last_notifications.last_event.timestamp() > event.created_at.unix_timestamp()
                    {
                        continue;
                    }

                    let beatmapset = self
                        .get_notify_beatmapset(connection, &beatmapset.url)
                        .await?;

                    let status = format!(
                        "[**{}**]({}) has submitted a new beatmap [**{} - {}**]({})",
                        new.username,
                        format_user_link(new.id),
                        beatmapset.0.artist,
                        beatmapset.0.title,
                        format_beatmap_link(None, beatmapset.0.id, None)
                    );

                    self.notify_beatmap_update(beatmapset, connection, linked_profile, &status)
                        .await?;

                    notified = true;
                }
                _ => {}
            }
        }

        if notified {
            let item = NewOsuNotification {
                id: linked_profile.osu_id,
                last_pp: last_notifications.last_pp,
                last_event: Utc::now(),
            };

            if let Err(why) =
                osu_notifications::update(connection, linked_profile.osu_id, &item).await
            {
                error!("Error occurred while running tracking loop: {}", why);
            }
        }

        Ok(())
    }

    async fn notify_beatmap_update(
        &mut self,
        beatmapset: (Beatmapset, Vec<(Beatmap, OsuFile)>),
        connection: &mut AsyncPgConnection,
        linked_profile: &LinkedOsuProfile,
        status: &str,
    ) -> Result<(), Error> {
        let mut embed = CreateEmbed::new();

        let description = format!("{}\n{}", status, format_beatmapset(beatmapset.1)?);

        embed = embed.image(beatmapset.0.cover).description(description);

        for guild_id in self.cache.guilds() {
            if let Ok(guild_channels) =
                osu_guild_channels::read(connection, i64::try_from(guild_id.get())?).await
            {
                if let Some(score_channels) = guild_channels.score_channel {
                    for score_channel in score_channels
                        .iter()
                        .flatten()
                        .copied()
                        .collect::<Vec<i64>>()
                    {
                        if guild_id
                            .member(
                                (Some(&self.cache), self.http.http()),
                                UserId::new(u64::try_from(linked_profile.id)?),
                            )
                            .await
                            .is_ok()
                        {
                            let builder = CreateMessage::new().embed(embed.clone());

                            ChannelId::from(u64::try_from(score_channel)?)
                                .send_message(&self.http, builder)
                                .await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
    async fn notify_leaderboard_score(
        &mut self,
        beatmap: &EventBeatmap,
        mode: &GameMode,
        new: &OsuUser,
        connection: &mut AsyncPgConnection,
        linked_profile: &LinkedOsuProfile,
    ) -> Result<(), Error> {
        let beatmap_info = get_beatmap_info(&format!("https://osu.ppy.sh{}", beatmap.url))?;

        let beatmap_id = u32::try_from(
            beatmap_info
                .beatmap_id
                .ok_or("Failed to get beatmap ID in notify_leaderboard_score")?,
        )?;

        let score = self
            .osu_client
            .beatmap_user_score(beatmap_id, u32::try_from(new.id)?)
            .mode(*mode)
            .await;

        let Ok(score) = score else {
            info!(
                "Couldn't retrieve user {} score on beatmap {} in notify_leaderboard_score",
                new.id, beatmap_id
            );
            return Ok(());
        };

        let score_id = score.score.id;

        let mut recent_scores = SCORE_NOTIFICATIONS
            .get_or_init(DashMap::new)
            .entry(linked_profile.osu_id)
            .or_default();

        if recent_scores.contains(&score_id) {
            return Ok(());
        }

        recent_scores.push(score_id);
        drop(recent_scores);

        let beatmap = get_beatmap(connection, self.osu_client.clone(), beatmap_id).await?;

        let pp = calculate(
            Some(&score.score),
            &beatmap.0,
            &beatmap.2,
            calculate_potential_acc(&score.score),
        )?;

        let footer = format_footer(&score.score, &beatmap.0, &pp)?;

        let author_text = format!("{} set a new leaderboard score!", new.username);

        let thumbnail = &beatmap.1.list_cover;

        let formatted_score = format!(
            "{}<t:{}:R>",
            format_new_score(
                &score.score,
                &beatmap.0,
                &beatmap.1,
                &pp,
                false,
                Some(&score.pos),
                None
            )?,
            score.score.ended_at.unix_timestamp()
        );

        let user_link = format_user_link(new.id);

        let title = format!(
            "{} - {} [{}]",
            beatmap.1.artist, beatmap.1.title, beatmap.0.version,
        );

        let title_url =
            format_beatmap_link(Some(beatmap.0.id), beatmap.1.id, Some(&mode.to_string()));

        for guild_id in self.cache.guilds() {
            if let Ok(guild_channels) =
                osu_guild_channels::read(connection, i64::try_from(guild_id.get())?).await
            {
                if let Some(score_channels) = guild_channels.score_channel {
                    for score_channel in score_channels
                        .iter()
                        .flatten()
                        .copied()
                        .collect::<Vec<i64>>()
                    {
                        if let Ok(member) = guild_id
                            .member(
                                (Some(&self.cache), self.http.http()),
                                UserId::new(u64::try_from(linked_profile.id)?),
                            )
                            .await
                        {
                            let color = member.colour(&self.cache).unwrap_or(BLUE);

                            let embed = create_embed(
                                color,
                                thumbnail,
                                &formatted_score,
                                &footer,
                                &new.avatar_url,
                                &author_text,
                                &user_link,
                                Some(title.clone()),
                                Some(title_url.clone()),
                            );

                            let builder = CreateMessage::new().embed(embed);

                            ChannelId::from(u64::try_from(score_channel)?)
                                .send_message(&self.http, builder)
                                .await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
