use crate::models::linked_osu_profiles::LinkedOsuProfile;
use crate::models::osu_notifications::NewOsuNotification;
use crate::models::osu_users::{NewOsuUser, OsuUser};
use crate::utils::db::osu_users::rosu_user_to_db;
use crate::utils::db::{linked_osu_profiles, osu_guild_channels, osu_notifications, osu_users};
use crate::utils::osu::caching::{get_beatmap, get_beatmapset};
use crate::utils::osu::calculate::calculate;
use crate::utils::osu::embeds::create_embed;
use crate::utils::osu::misc::{
    calculate_potential_acc, gamemode_from_string, get_stat_diff, is_playing, DiffTypes,
};
use crate::utils::osu::misc_format::{format_diff, format_potential_string, format_user_link};
use crate::utils::osu::regex::get_beatmap_info;
use crate::utils::osu::score_format::{format_new_score, format_score_list};
use crate::{Error, Pool};
use chrono::Utc;
use dashmap::DashMap;
use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use lazy_static::lazy_static;
use poise::serenity_prelude::model::colour::colours::roles::BLUE;
use poise::serenity_prelude::{ChannelId, Context, CreateMessage};
use rosu_v2::model::GameMode;
use rosu_v2::prelude::{EventBeatmap, EventType, Score};
use rosu_v2::Osu;
use std::env;
use std::num::NonZeroU64;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::error;

lazy_static! {
    static ref PP_THRESHOLD: f64 = env::var("PP_THRESHOLD")
        .unwrap_or_else(|_| String::from("0.1"))
        .parse::<f64>()
        .expect("Failed to parse tracking update interval.");
}

lazy_static! {
    static ref UPDATE_INTERVAL: u64 = env::var("UPDATE_INTERVAL")
        .unwrap_or_else(|_| String::from("30"))
        .parse::<u64>()
        .expect("Failed to parse tracking update interval.");
}

lazy_static! {
    static ref NOT_PLAYING_SKIP: i32 = env::var("NOT_PLAYING_SKIP")
        .unwrap_or_else(|_| String::from("10"))
        .parse::<i32>()
        .expect("Failed to parse tracking not playing skip.");
}

lazy_static! {
    static ref SCORE_NOTIFICATIONS: DashMap<i64, Vec<u64>> = DashMap::new();
}

pub struct OsuTracker {
    pub ctx: Context,
    pub osu_client: Arc<Osu>,
    pub pool: Pool<ConnectionManager<PgConnection>>,
    pub shut_down: bool,
}
impl OsuTracker {
    pub async fn tracking_loop(&mut self) -> Result<(), Error> {
        loop {
            sleep(Duration::from_secs(*UPDATE_INTERVAL)).await;
            let connection = &mut self.pool.get()?;
            let profiles = match linked_osu_profiles::get_all(connection) {
                Ok(profiles) => profiles,
                Err(why) => {
                    error!("Failed to get linked osu profiles {}", why);
                    continue;
                }
            };
            for profile in profiles {
                if let Err(why) = self.update_user_data(&profile, connection).await {
                    error!("Error occurred while running tracking loop: {}", why);
                    continue;
                }
            }
        }
    }

    async fn update_user_data(
        &mut self,
        linked_profile: &LinkedOsuProfile,
        connection: &mut PgConnection,
    ) -> Result<(), Error> {
        let user = match self.ctx.cache.user(linked_profile.id as u64) {
            Some(user) => user.clone(),
            _ => return Ok(()),
        };

        if let Ok(mut profile) = osu_users::read(connection, linked_profile.osu_id) {
            profile.ticks += 1;
            if is_playing(&self.ctx, user.id, linked_profile.home_guild)?
                || (f64::from(profile.ticks) % f64::from(*NOT_PLAYING_SKIP)) == 0.0
            {
                let Ok(osu_profile) = self
                    .osu_client
                    .user(linked_profile.osu_id as u32)
                    .mode(gamemode_from_string(&linked_profile.mode).ok_or("Failed to parse gamemode in update_user_data function")?)
                    .await else { return Ok(()) };
                let new = osu_users::create(
                    connection,
                    &rosu_user_to_db(osu_profile, Some(profile.ticks))?,
                )?;

                if let Err(why) = self
                    .notify_pp(&profile, &new, connection, linked_profile)
                    .await
                {
                    error!("Error occurred while running tracking loop: {}", why);
                    return Ok(());
                }

                if let Err(why) = self.notify_recent(&new, connection, linked_profile).await {
                    error!("Error occurred while running tracking loop: {}", why);
                    return Ok(());
                }
            } else {
                let user_update = NewOsuUser {
                    id: profile.id,
                    username: profile.username,
                    avatar_url: profile.avatar_url,
                    country_code: profile.country_code,
                    mode: profile.mode,
                    pp: profile.pp,
                    accuracy: profile.accuracy,
                    country_rank: profile.country_rank,
                    global_rank: profile.global_rank,
                    max_combo: profile.max_combo,
                    ticks: profile.ticks,
                    ranked_score: profile.ranked_score,
                    time_cached: profile.time_cached,
                };

                osu_users::create(connection, &user_update)?;
            }
        } else {
            let Ok(osu_profile) = self
                .osu_client
                .user(linked_profile.osu_id as u32)
                .mode(gamemode_from_string(&linked_profile.mode).ok_or("Failed to parse gamemode in update_user_data function")?)
                .await else { return Ok(()) };

            osu_users::create(connection, &rosu_user_to_db(osu_profile, None)?)?;
        }

        Ok(())
    }

    async fn notify_pp(
        &mut self,
        old: &OsuUser,
        new: &OsuUser,
        connection: &mut PgConnection,
        linked_profile: &LinkedOsuProfile,
    ) -> Result<(), Error> {
        if get_stat_diff(old, new, &DiffTypes::Pp) < *PP_THRESHOLD {
            return Ok(());
        }
        let new_scores = self
            .get_new_score(new.id, linked_profile, connection)
            .await?;
        if new_scores.is_empty() {
            return Ok(());
        } else if new_scores.len() == 1 {
            self.notify_single_score(&new_scores, linked_profile, new, old, connection)
                .await?;
        } else {
            self.notify_multiple_scores(&new_scores, linked_profile, new, old, connection)
                .await?;
        };

        Ok(())
    }

    async fn notify_multiple_scores(
        &mut self,
        new_scores: &[(Score, usize)],
        linked_profile: &LinkedOsuProfile,
        new: &OsuUser,
        old: &OsuUser,
        connection: &mut PgConnection,
    ) -> Result<(), Error> {
        let mut recent_scores = SCORE_NOTIFICATIONS
            .entry(linked_profile.osu_id)
            .or_default();

        let mut to_notify: Vec<(Score, usize)> = Vec::new();

        let gamemode = gamemode_from_string(&linked_profile.mode)
            .ok_or("Failed to get parse gamemode in notify_multiple_scores function")?;

        for score in new_scores.iter() {
            let score_id = score
                .0
                .score_id
                .ok_or("Failed to get score id in notify_multiple_scores function")?;

            if recent_scores.value().contains(&score_id) {
                continue;
            }
            recent_scores.push(score_id);

            let api_score = self.osu_client.score(score_id, gamemode).await?;

            to_notify.push((api_score.clone(), score.1));
        }

        if to_notify.is_empty() {
            return Ok(());
        }

        let author_text = format!("{} set a new best scores", &new.username);

        let thumbnail = new.avatar_url.clone();

        let formatted_score = format!(
            "{}\n{}",
            format_score_list(
                connection,
                self.osu_client.clone(),
                &to_notify,
                None,
                None,
                None,
                None
            )
            .await?,
            format_diff(new, old, gamemode)?
        );

        self.send_score_notifications(
            connection,
            linked_profile,
            &thumbnail,
            &formatted_score,
            "",
            &author_text,
            new,
        )
        .await?;

        Ok(())
    }

    async fn notify_single_score(
        &mut self,
        new_scores: &[(Score, usize)],
        linked_profile: &LinkedOsuProfile,
        new: &OsuUser,
        old: &OsuUser,
        connection: &mut PgConnection,
    ) -> Result<(), Error> {
        let score = &new_scores[0];

        let score_id = score
            .0
            .score_id
            .ok_or("Failed to get score_id in notify_single_score function")?;

        let gamemode = gamemode_from_string(&linked_profile.mode)
            .ok_or("Failed to parse gamemode in notify_single_score function")?;

        if let Some(mut recent_scores) = SCORE_NOTIFICATIONS.get_mut(&linked_profile.osu_id) {
            if recent_scores.value().contains(&score_id) {
                return Ok(());
            }
            recent_scores.push(score_id);
        } else {
            SCORE_NOTIFICATIONS.insert(linked_profile.osu_id, vec![score_id]);
        };

        let beatmap = get_beatmap(connection, self.osu_client.clone(), score.0.map_id).await?;

        let beatmapset = get_beatmapset(
            connection,
            self.osu_client.clone(),
            beatmap.beatmapset_id as u32,
        )
        .await?;

        let pp = calculate(&score.0, &beatmap, calculate_potential_acc(&score.0)).await;
        let author_text = format!(
            "{} set a new best score (#{}/{})",
            &new.username, score.1, 100
        );
        let potential_string: String;
        let pp = if let Ok(pp) = pp {
            potential_string = format_potential_string(&pp)?;
            Some(pp)
        } else {
            potential_string = String::new();
            None
        };

        let api_score = self.osu_client.score(score_id, gamemode).await?;

        let thumbnail = beatmapset.list_cover.clone();
        let formatted_score = format!(
            "{}{}\n<t:{}:R>",
            format_new_score(&api_score, &beatmap, &beatmapset, &pp, None)?,
            format_diff(new, old, gamemode)?,
            score.0.ended_at.unix_timestamp()
        );

        self.send_score_notifications(
            connection,
            linked_profile,
            &thumbnail,
            &formatted_score,
            &potential_string,
            &author_text,
            new,
        )
        .await?;

        Ok(())
    }

    async fn send_score_notifications(
        &mut self,
        connection: &mut PgConnection,
        linked_profile: &LinkedOsuProfile,
        thumbnail: &str,
        formatted_score: &str,
        footer: &str,
        author_text: &str,
        new: &OsuUser,
    ) -> Result<(), Error> {
        for guild_id in self.ctx.cache.guilds() {
            if let Ok(guild_channels) =
                osu_guild_channels::read(connection, guild_id.0.get() as i64)
            {
                if let Some(score_channel) = guild_channels.score_channel {
                    if let Ok(member) = guild_id.member(&self.ctx, linked_profile.id as u64).await {
                        let color = member.colour(&self.ctx).unwrap_or(BLUE);

                        let embed = create_embed(
                            color,
                            thumbnail,
                            formatted_score,
                            footer,
                            &new.avatar_url,
                            author_text,
                            &format_user_link(new.id),
                        );

                        let builder = CreateMessage::new().embed(embed);

                        ChannelId(NonZeroU64::try_from(score_channel as u64)?)
                            .send_message(&self.ctx, builder)
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn get_new_score(
        &mut self,
        osu_id: i64,
        linked_profile: &LinkedOsuProfile,
        connection: &mut PgConnection,
    ) -> Result<Vec<(Score, usize)>, Error> {
        let last_notifications = if let Ok(updates) = osu_notifications::read(connection, osu_id) {
            updates
        } else {
            let item = NewOsuNotification {
                id: osu_id,
                last_pp: Utc::now(),
                last_event: Utc::now(),
            };
            osu_notifications::create(connection, &item)?
        };

        let mut new_scores = Vec::new();

        let best_scores = self
            .osu_client
            .user_scores(osu_id as u32)
            .best()
            .mode(
                gamemode_from_string(&linked_profile.mode)
                    .ok_or("Failed to parse gamemode in get_new_score function")?,
            )
            .limit(100)
            .await
            .unwrap_or_default();

        for (pos, score) in best_scores.iter().enumerate() {
            if score.ended_at.unix_timestamp() > last_notifications.last_pp.timestamp() {
                new_scores.push((score.clone(), pos + 1));
            }
        }

        if !new_scores.is_empty() {
            let item = NewOsuNotification {
                id: osu_id,
                last_pp: Utc::now(),
                last_event: last_notifications.last_event,
            };

            if let Err(why) = osu_notifications::update(connection, osu_id, &item) {
                error!("Error occurred while running tracking loop: {}", why);
            };
        }

        Ok(new_scores)
    }

    async fn notify_recent(
        &mut self,
        new: &OsuUser,
        connection: &mut PgConnection,
        linked_profile: &LinkedOsuProfile,
    ) -> Result<(), Error> {
        let last_notifications =
            if let Ok(updates) = osu_notifications::read(connection, linked_profile.osu_id) {
                updates
            } else {
                let item = NewOsuNotification {
                    id: linked_profile.osu_id,
                    last_pp: Utc::now(),
                    last_event: Utc::now(),
                };
                osu_notifications::create(connection, &item)?
            };

        let mut recent_events = self.osu_client.recent_events(new.id as u32).await?;
        recent_events.reverse();

        let mut notified = false;

        for event in &recent_events {
            if let EventType::Rank {
                grade: _grade,
                rank,
                mode,
                beatmap,
                user: _user,
            } = &event.event_type
            {
                if last_notifications.last_event.timestamp() > event.created_at.unix_timestamp() {
                    continue;
                }

                if rank > &50 {
                    continue;
                }

                self.notify_leaderboard_score(beatmap, mode, new, connection, linked_profile)
                    .await?;

                notified = true;
            }
        }

        if notified {
            let item = NewOsuNotification {
                id: linked_profile.osu_id,
                last_pp: last_notifications.last_pp,
                last_event: Utc::now(),
            };

            if let Err(why) = osu_notifications::update(connection, linked_profile.osu_id, &item) {
                error!("Error occurred while running tracking loop: {}", why);
            };
        }

        Ok(())
    }

    async fn notify_leaderboard_score(
        &mut self,
        beatmap: &EventBeatmap,
        mode: &GameMode,
        new: &OsuUser,
        connection: &mut PgConnection,
        linked_profile: &LinkedOsuProfile,
    ) -> Result<(), Error> {
        let mut recent_scores = SCORE_NOTIFICATIONS
            .entry(linked_profile.osu_id)
            .or_default();

        let beatmap_info = get_beatmap_info(&format!("https://osu.ppy.sh{}", beatmap.url))?;

        let beatmap_id = beatmap_info
            .beatmap_id
            .ok_or("Failed to get beatmap ID in notify_leaderboard_score")?
            as u32;

        let score = self
            .osu_client
            .beatmap_user_score(beatmap_id, new.id as u32)
            .mode(*mode)
            .await?;

        let score_id = score
            .score
            .score_id
            .ok_or("Failed to get score_id in notify_leaderboard_score")?;

        if recent_scores.contains(&score_id) {
            return Ok(());
        }

        recent_scores.push(score_id);

        let beatmap = get_beatmap(connection, self.osu_client.clone(), beatmap_id).await?;

        let beatmapset = get_beatmapset(
            connection,
            self.osu_client.clone(),
            beatmap.beatmapset_id as u32,
        )
        .await?;

        let pp = calculate(
            &score.score,
            &beatmap,
            calculate_potential_acc(&score.score),
        )
        .await;

        let potential_string: String;
        let pp = if let Ok(pp) = pp {
            potential_string = format_potential_string(&pp)?;
            Some(pp)
        } else {
            potential_string = String::new();
            None
        };

        let author_text = &format!("{} set a new leaderboard score!", new.username);

        let thumbnail = &beatmapset.list_cover;

        let footer = &potential_string;

        let formatted_score = &format!(
            "{}<t:{}:R>",
            format_new_score(&score.score, &beatmap, &beatmapset, &pp, Some(&score.pos))?,
            score.score.ended_at.unix_timestamp()
        );

        for guild_id in self.ctx.cache.guilds() {
            if let Ok(guild_channels) =
                osu_guild_channels::read(connection, guild_id.0.get() as i64)
            {
                if let Some(score_channel) = guild_channels.score_channel {
                    if let Ok(member) = guild_id.member(&self.ctx, linked_profile.id as u64).await {
                        let color = member.colour(&self.ctx).unwrap_or(BLUE);

                        let embed = create_embed(
                            color,
                            thumbnail,
                            formatted_score,
                            footer,
                            &new.avatar_url,
                            author_text,
                            &format_user_link(new.id),
                        );

                        let builder = CreateMessage::new().embed(embed);

                        ChannelId(NonZeroU64::try_from(score_channel as u64)?)
                            .send_message(&self.ctx, builder)
                            .await?;
                    }
                }
            }
        }
        Ok(())
    }
}
