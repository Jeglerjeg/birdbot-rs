use crate::Error;
use crate::models::linked_osu_profiles::LinkedOsuProfile;
use crate::models::osu_notifications::{NewOsuNotification, OsuNotification};
use crate::models::osu_users::OsuUser;
use crate::utils::db::{linked_osu_profiles, osu_notifications};
use crate::utils::db::{osu_guild_channels, osu_users};
use crate::utils::osu::caching::get_beatmap;
use crate::utils::osu::calculate::calculate;
use crate::utils::osu::embeds::create_embed;
use crate::utils::osu::misc::{
    add_profile_data, calculate_potential_acc, gamemode_from_string, get_score_position,
};
use crate::utils::osu::misc_format::{
    format_beatmap_link, format_diff, format_footer, format_user_link,
};
use crate::utils::osu::score_format::format_new_score;
use chrono::{TimeZone, Utc};
use dashmap::DashMap;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use futures_util::{SinkExt, StreamExt};
use mobc::Pool;
use rosu_pp::model::mods::rosu_mods::GameMode;
use rosu_v2::Osu;
use rosu_v2::prelude::Score;
use serenity::all::colours::roles::BLUE;
use serenity::all::{Cache, CacheHttp, ChannelId, CreateMessage, Http, UserId};
use std::env;
use std::sync::{Arc, LazyLock};
use tokio_tungstenite::tungstenite::Message;
use tracing::error;

pub static TRACKED_USERS: LazyLock<DashMap<i64, Vec<i64>>> = LazyLock::new(DashMap::new);

static WS_URL: LazyLock<String> = LazyLock::new(|| {
    env::var("SCORES_WS_URL").unwrap_or_else(|_| String::from("ws://127.0.0.1:7727"))
});

async fn init_tracked_users(connection: &mut AsyncPgConnection) -> Result<(), Error> {
    let profiles = linked_osu_profiles::get_all(connection).await?;

    let tracked_users = &*TRACKED_USERS;

    for profile in profiles {
        if let Some(mut tracked_user) = tracked_users.get_mut(&profile.osu_id) {
            tracked_user.push(profile.id);
            drop(tracked_user);
        } else {
            tracked_users.insert(profile.osu_id, vec![profile.id]);
        }
    }

    Ok(())
}

pub fn add_tracked_user(member_id: i64, osu_id: i64) {
    let tracked_users = &*TRACKED_USERS;

    if let Some(mut tracked_user) = tracked_users.get_mut(&osu_id) {
        tracked_user.push(member_id);
    } else {
        tracked_users.insert(osu_id, vec![member_id]);
    }
}

pub fn remove_tracked_user(member_id: i64, osu_id: i64) {
    let tracked_users = &*TRACKED_USERS;

    if let Some(mut tracked_user) = tracked_users.get_mut(&osu_id) {
        tracked_user.retain(|&x| x != member_id);
        if tracked_user.is_empty() {
            tracked_users.remove(&osu_id);
        }
    }
}

fn check_top100(score: &Score, score_list: &mut Vec<Score>) -> bool {
    score_list.sort_by(|a, b| b.pp.unwrap_or(0.0).total_cmp(&a.pp.unwrap_or(0.0)));

    if score.pp.unwrap_or(0.0) < score_list[score_list.len() - 1].pp.unwrap_or(0.0) {
        return false;
    }

    for api_score in score_list {
        if (api_score.map_id == score.map_id)
            && (score.pp.unwrap_or(0.0) < api_score.pp.unwrap_or(0.0))
        {
            return false;
        }
    }

    true
}

pub struct ScoresWs {
    pub cache: Arc<Cache>,
    pub http: Arc<Http>,
    pub osu_client: Arc<Osu>,
    pub pool: Pool<AsyncDieselConnectionManager<AsyncPgConnection>>,
}
impl ScoresWs {
    pub async fn connect_websocket(&mut self) -> Result<(), Error> {
        let mut connection = self.pool.get().await?;
        init_tracked_users(&mut connection).await?;
        drop(connection);

        let url = &*WS_URL;

        let (ws_stream, _) = tokio_tungstenite::connect_async(url).await?;

        let (mut write, mut read) = ws_stream.split();

        write.send(Message::from("connect")).await?;

        self.process_scores(&mut read).await;

        Ok(())
    }

    async fn get_osu_user(
        &mut self,
        connection: &mut AsyncPgConnection,
        linked_profile: &LinkedOsuProfile,
        mode: GameMode,
    ) -> Result<OsuUser, Error> {
        if let Ok(osu_user) = osu_users::read(connection, linked_profile.osu_id).await {
            Ok(osu_user)
        } else {
            add_profile_data(
                self.osu_client.clone(),
                u32::try_from(linked_profile.osu_id)?,
                mode,
                connection,
            )
            .await
        }
    }

    async fn process_scores<
        S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
    >(
        &mut self,
        stream: &mut S,
    ) {
        let con_result = &mut self.pool.get().await;
        let connection = match con_result {
            Ok(connection) => connection,
            Err(why) => {
                error!("{}", why);
                return;
            }
        };
        while let Some(res) = stream.next().await {
            let Message::Binary(data) = (match res {
                Ok(data) => data,
                Err(why) => {
                    error!("{}", why);
                    continue;
                }
            }) else {
                continue;
            };

            let score: Score = match serde_json::from_slice(&data) {
                Ok(score) => score,
                Err(err) => {
                    error!("{}", err);
                    continue;
                }
            };

            let tracked_user = TRACKED_USERS.get(&i64::from(score.user_id));

            if tracked_user.is_none() {
                drop(tracked_user);
                continue;
            }

            let users = tracked_user.unwrap().clone();

            let last_notifications = if let Ok(updates) =
                osu_notifications::read(connection, i64::from(score.user_id)).await
            {
                updates
            } else {
                let item = NewOsuNotification {
                    id: i64::from(score.user_id),
                    last_pp: Utc::now(),
                    last_event: Utc::now(),
                };
                match osu_notifications::create(connection, &item).await {
                    Ok(last_notifications) => last_notifications,
                    Err(why) => {
                        error!("{}", why);
                        continue;
                    }
                }
            };

            if score.ended_at.unix_timestamp() <= last_notifications.last_pp.timestamp() {
                continue;
            }

            for osu_user_id in users {
                let linked_profile = match linked_osu_profiles::read(connection, osu_user_id).await
                {
                    Ok(profile) => profile,
                    Err(why) => {
                        error!("{}", why);
                        continue;
                    }
                };

                let mode = match gamemode_from_string(&linked_profile.mode) {
                    None => {
                        error!(
                            "Couldn't convert mode {} for user {}",
                            linked_profile.mode, linked_profile.id
                        );
                        continue;
                    }
                    Some(mode) => mode,
                };

                if score.mode != mode {
                    continue;
                }

                let osu_user = match self.get_osu_user(connection, &linked_profile, mode).await {
                    Ok(osu_user) => osu_user,
                    Err(why) => {
                        error!("{}", why);
                        continue;
                    }
                };

                if (score.pp.unwrap_or(0.0) as f64) < osu_user.min_pp {
                    continue;
                }

                if let Err(why) = self
                    .check_notify(
                        score.clone(),
                        &osu_user,
                        linked_profile,
                        &last_notifications,
                        connection,
                    )
                    .await
                {
                    error!("{}", why);
                }
            }
        }
    }

    async fn check_notify(
        &mut self,
        score: Score,
        old: &OsuUser,
        linked_profile: LinkedOsuProfile,
        last_notifications: &OsuNotification,
        connection: &mut AsyncPgConnection,
    ) -> Result<(), Error> {
        let mut best_scores = match self
            .osu_client
            .user_scores(score.user_id)
            .best()
            .mode(score.mode)
            .limit(100)
            .await
        {
            Ok(best_scores) => best_scores,
            Err(why) => {
                error!("{}", why);
                return Ok(());
            }
        };

        if check_top100(&score, &mut best_scores) {
            let score_position = get_score_position(&score, best_scores)?;

            let score = self.osu_client.score(score.id).await.unwrap_or(score);

            let new = add_profile_data(
                self.osu_client.clone(),
                u32::try_from(linked_profile.osu_id)?,
                score.mode,
                connection,
            )
            .await?;

            self.notify_single_score(
                &(score, score_position),
                &linked_profile,
                &new,
                old,
                last_notifications,
                connection,
            )
            .await?;
        }
        Ok(())
    }

    async fn notify_single_score(
        &mut self,
        score: &(Score, usize),
        linked_profile: &LinkedOsuProfile,
        new: &OsuUser,
        old: &OsuUser,
        last_notifications: &OsuNotification,
        connection: &mut AsyncPgConnection,
    ) -> Result<(), Error> {
        let score_id = score.0.id;

        let gamemode = gamemode_from_string(&linked_profile.mode)
            .ok_or("Failed to parse gamemode in notify_single_score function")?;

        let mut recent_scores = crate::utils::osu::tracking::SCORE_NOTIFICATIONS
            .get_or_init(DashMap::new)
            .entry(linked_profile.osu_id)
            .or_default();

        if recent_scores.value().contains(&score_id) {
            return Ok(());
        }
        recent_scores.push(score_id);
        drop(recent_scores);

        let beatmap = get_beatmap(connection, self.osu_client.clone(), score.0.map_id).await?;

        let pp = calculate(
            Some(&score.0),
            &beatmap.0,
            &beatmap.2,
            calculate_potential_acc(&score.0),
        )?;
        let author_text = format!(
            "{} set a new best score (#{}/{})",
            &new.username, score.1, 100
        );
        let footer = format_footer(&score.0, &beatmap.0, &pp)?;

        let title = format!(
            "{} - {} [{}]",
            beatmap.1.artist, beatmap.1.title, beatmap.0.version,
        );

        let title_url = format_beatmap_link(
            Some(beatmap.0.id),
            beatmap.1.id,
            Some(&score.0.mode.to_string()),
        );

        let thumbnail = beatmap.1.list_cover.clone();
        let formatted_score = format!(
            "{}{}\n<t:{}:R>",
            format_new_score(&score.0, &beatmap.0, &beatmap.1, &pp, false, None, None)?,
            format_diff(new, old, gamemode)?,
            score.0.ended_at.unix_timestamp()
        );

        let item = NewOsuNotification {
            id: linked_profile.osu_id,
            last_pp: Utc.timestamp_nanos(i64::try_from(score.0.ended_at.unix_timestamp_nanos())?),
            last_event: last_notifications.last_event,
        };

        if let Err(why) = osu_notifications::update(connection, linked_profile.osu_id, &item).await
        {
            error!("Error occurred while running scores-ws: {}", why);
        }

        self.send_score_notifications(
            connection,
            linked_profile,
            &thumbnail,
            &formatted_score,
            &footer,
            &author_text,
            Some(title),
            Some(title_url),
            new,
        )
        .await?;

        Ok(())
    }

    async fn send_score_notifications(
        &mut self,
        connection: &mut AsyncPgConnection,
        linked_profile: &LinkedOsuProfile,
        thumbnail: &str,
        formatted_score: &str,
        footer: &str,
        author_text: &str,
        title: Option<String>,
        title_url: Option<String>,
        new: &OsuUser,
    ) -> Result<(), Error> {
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

                            let user_link = format_user_link(new.id);

                            let embed = create_embed(
                                color,
                                thumbnail,
                                formatted_score,
                                footer,
                                &new.avatar_url,
                                author_text,
                                &user_link,
                                title.clone(),
                                title_url.clone(),
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
