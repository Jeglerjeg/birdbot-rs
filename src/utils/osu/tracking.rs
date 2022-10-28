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
use poise::serenity_prelude;
use rosu_v2::prelude::{EventType, Score};
use rosu_v2::Osu;
use serenity_prelude::model::colour::colours::roles::BLUE;
use serenity_prelude::{ChannelId, CreateMessage};
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
    pub ctx: serenity_prelude::Context,
    pub osu_client: Arc<Osu>,
    pub pool: Pool<ConnectionManager<PgConnection>>,
    pub shut_down: bool,
}
impl OsuTracker {
    pub async fn tracking_loop(&mut self) {
        loop {
            sleep(Duration::from_secs(*UPDATE_INTERVAL)).await;
            let connection = &mut self.pool.get().unwrap();
            let profiles = match linked_osu_profiles::get_all(connection) {
                Ok(profiles) => profiles,
                Err(why) => {
                    error!("Failed to get linked osu profiles {}", why);
                    continue;
                }
            };
            for profile in profiles {
                if let Err(why) = self.update_user_data(&profile, connection).await {
                    error!("Error occured while running tracking loop: {}", why);
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
            if is_playing(&self.ctx, user.id, linked_profile.home_guild)
                || (profile.ticks as f64 % *NOT_PLAYING_SKIP as f64) == 0.0
            {
                let osu_profile = match self
                    .osu_client
                    .user(linked_profile.osu_id as u32)
                    .mode(gamemode_from_string(&linked_profile.mode).unwrap())
                    .await
                {
                    Ok(profile) => profile,
                    Err(_) => return Ok(()),
                };
                let new = osu_users::create(
                    connection,
                    &rosu_user_to_db(osu_profile, Some(profile.ticks)),
                )?;

                if let Err(why) = self
                    .notify_pp(&profile, &new, connection, linked_profile)
                    .await
                {
                    error!("Error occured while running tracking loop: {}", why);
                    return Ok(());
                }

                if let Err(why) = self.notify_recent(&new, connection, linked_profile).await {
                    error!("Error occured while running tracking loop: {}", why);
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
            let osu_profile = match self
                .osu_client
                .user(linked_profile.osu_id as u32)
                .mode(gamemode_from_string(&linked_profile.mode).unwrap())
                .await
            {
                Ok(proile) => proile,
                Err(_) => return Ok(()),
            };

            osu_users::create(connection, &rosu_user_to_db(osu_profile, None))?;
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
        let author_text: String;
        let formatted_score: String;
        let footer: String;
        let thumbnail: String;

        if get_stat_diff(old, new, &DiffTypes::Pp) < *PP_THRESHOLD {
            return Ok(());
        }
        let new_scores = self.get_new_score(new.id, linked_profile, connection).await;
        if new_scores.is_empty() {
            return Ok(());
        } else if new_scores.len() == 1 {
            let score = &new_scores[0];

            if let Some(mut recent_scores) = SCORE_NOTIFICATIONS.get_mut(&linked_profile.osu_id) {
                if recent_scores.value().contains(&score.0.score_id.unwrap()) {
                    return Ok(());
                }
                recent_scores.push(score.0.score_id.unwrap());
            } else {
                SCORE_NOTIFICATIONS.insert(linked_profile.osu_id, vec![score.0.score_id.unwrap()]);
            };

            let beatmap = get_beatmap(
                connection,
                self.osu_client.clone(),
                score.0.map.as_ref().unwrap().map_id,
            )
            .await?;

            let beatmapset = get_beatmapset(
                connection,
                self.osu_client.clone(),
                beatmap.beatmapset_id as u32,
            )
            .await?;

            let pp = calculate(&score.0, &beatmap, calculate_potential_acc(&score.0)).await;
            author_text = format!(
                "{} set a new best score (#{}/{})",
                &new.username, score.1, 100
            );
            let potential_string: String;
            let pp = if let Ok(pp) = pp {
                potential_string = format_potential_string(&pp);
                Some(pp)
            } else {
                potential_string = String::new();
                None
            };

            thumbnail = beatmapset.list_cover.clone();
            formatted_score = format!(
                "{}{}\n<t:{}:R>",
                format_new_score(&score.0, &beatmap, &beatmapset, &pp, None),
                format_diff(
                    new,
                    old,
                    gamemode_from_string(&linked_profile.mode).unwrap()
                ),
                score.0.ended_at.unix_timestamp()
            );

            footer = potential_string;
        } else {
            let mut recent_scores = SCORE_NOTIFICATIONS
                .entry(linked_profile.osu_id)
                .or_insert(vec![]);

            let mut to_notify: Vec<(Score, usize)> = Vec::new();

            for score in &new_scores {
                if recent_scores.value().contains(&score.0.score_id.unwrap()) {
                    continue;
                } else {
                    recent_scores.push(score.0.score_id.unwrap());
                    to_notify.push(score.clone());
                };
            }

            if to_notify.is_empty() {
                return Ok(());
            }

            author_text = format!("{} set a new best scores", &new.username);

            thumbnail = new.avatar_url.clone();

            footer = String::new();

            formatted_score = format!(
                "{}\n{}",
                format_score_list(connection, self.osu_client.clone(), &to_notify, None, None)
                    .await?,
                format_diff(
                    new,
                    old,
                    gamemode_from_string(&linked_profile.mode).unwrap()
                )
            );
        };

        for guild_id in self.ctx.cache.guilds() {
            if let Ok(guild_channels) =
                osu_guild_channels::read(connection, guild_id.0.get() as i64)
            {
                if let Some(score_channel) = guild_channels.score_channel {
                    if let Ok(member) = guild_id.member(&self.ctx, linked_profile.id as u64).await {
                        let color = member.colour(&self.ctx).unwrap_or(BLUE);

                        let embed = create_embed(
                            color,
                            &thumbnail,
                            &formatted_score,
                            &footer,
                            &new.avatar_url,
                            &author_text,
                            &format_user_link(new.id),
                        );

                        let builder = CreateMessage::new().embed(embed);

                        ChannelId(NonZeroU64::try_from(score_channel as u64).unwrap())
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
    ) -> Vec<(Score, usize)> {
        let last_notifications = if let Ok(updates) = osu_notifications::read(connection, osu_id) {
            updates
        } else {
            let item = NewOsuNotification {
                id: osu_id,
                last_pp: Utc::now(),
                last_event: Utc::now(),
            };
            osu_notifications::create(connection, &item).unwrap()
        };

        let mut new_scores = Vec::new();

        let best_scores = self
            .osu_client
            .user_scores(osu_id as u32)
            .best()
            .mode(gamemode_from_string(&linked_profile.mode).unwrap())
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
                error!("Error occured while running tracking loop: {}", why);
            };
        }

        new_scores
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
                osu_notifications::create(connection, &item).unwrap()
            };

        let mut recent_events = self.osu_client.recent_events(new.id as u32).await?;
        recent_events.reverse();

        let mut recent_scores = SCORE_NOTIFICATIONS
            .entry(linked_profile.osu_id)
            .or_insert(vec![]);

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

                let beatmap_info = get_beatmap_info(&format!("https://osu.ppy.sh{}", beatmap.url));

                let score = self
                    .osu_client
                    .beatmap_user_score(beatmap_info.beatmap_id.unwrap() as u32, new.id as u32)
                    .mode(*mode)
                    .await
                    .unwrap();

                if recent_scores.contains(&score.score.score_id.unwrap()) {
                    continue;
                } else {
                    recent_scores.push(score.score.score_id.unwrap());
                }

                let beatmap = get_beatmap(
                    connection,
                    self.osu_client.clone(),
                    beatmap_info.beatmap_id.unwrap() as u32,
                )
                .await?;

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
                    potential_string = format_potential_string(&pp);
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
                    format_new_score(&score.score, &beatmap, &beatmapset, &pp, Some(&score.pos)),
                    score.score.ended_at.unix_timestamp()
                );

                for guild_id in self.ctx.cache.guilds() {
                    if let Ok(guild_channels) =
                        osu_guild_channels::read(connection, guild_id.0.get() as i64)
                    {
                        if let Some(score_channel) = guild_channels.score_channel {
                            if let Ok(member) =
                                guild_id.member(&self.ctx, linked_profile.id as u64).await
                            {
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

                                ChannelId(NonZeroU64::try_from(score_channel as u64).unwrap())
                                    .send_message(&self.ctx, builder)
                                    .await?;
                            }
                        }
                    }
                }
            };
        }

        let item = NewOsuNotification {
            id: linked_profile.osu_id,
            last_pp: last_notifications.last_pp,
            last_event: Utc::now(),
        };

        if let Err(why) = osu_notifications::update(connection, linked_profile.osu_id, &item) {
            error!("Error occured while running tracking loop: {}", why);
        };

        Ok(())
    }
}
