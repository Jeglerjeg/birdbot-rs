use crate::models::osu_users::NewOsuUser;
use crate::{Error, Pool};
use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use lazy_static::lazy_static;
use poise::serenity_prelude;
use rosu_v2::Osu;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::error;

use crate::utils::db::osu_users::rosu_user_to_db;
use crate::utils::db::{linked_osu_profiles, osu_users};
use crate::utils::osu::misc::gamemode_from_string;

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

pub struct OsuTracker {
    pub ctx: serenity_prelude::Context,
    pub osu_client: Arc<Osu>,
    pub pool: Pool<ConnectionManager<PgConnection>>,
    pub shut_down: bool,
}
impl OsuTracker {
    pub async fn tracking_loop(&mut self) {
        while !self.shut_down {
            sleep(Duration::from_secs(*UPDATE_INTERVAL)).await;
            let connection = &mut self.pool.get().unwrap();
            let profiles = match linked_osu_profiles::get_all(connection) {
                Ok(profiles) => profiles,
                Err(why) => {
                    error!("Failed to get linked osu profiles {}", why);
                    self.shut_down = true;
                    continue;
                }
            };
            for profile in profiles {
                if let Err(why) = self
                    .update_user_data(profile.id, profile.osu_id, profile.mode, connection)
                    .await
                {
                    error!("Error occured while running tracking loop: {}", why);
                }
            }
        }
    }

    async fn update_user_data(
        &mut self,
        discord_id: i64,
        osu_id: i64,
        mode: String,
        connection: &mut PgConnection,
    ) -> Result<(), Error> {
        match self.ctx.cache.user(discord_id as u64) {
            Some(user) => user,
            _ => return Ok(()),
        };

        if let Ok(mut profile) = osu_users::read(connection, osu_id) {
            profile.ticks += 1;
            if (profile.ticks % *NOT_PLAYING_SKIP) == 0 {
                let osu_profile = match self
                    .osu_client
                    .user(osu_id as u32)
                    .mode(gamemode_from_string(&mode).unwrap())
                    .await
                {
                    Ok(profile) => profile,
                    Err(_) => return Ok(()),
                };
                osu_users::create(
                    connection,
                    &rosu_user_to_db(osu_profile, Some(profile.ticks)),
                )?;
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
                .user(osu_id as u32)
                .mode(gamemode_from_string(&mode).unwrap())
                .await
            {
                Ok(proile) => proile,
                Err(_) => return Ok(()),
            };

            osu_users::create(connection, &rosu_user_to_db(osu_profile, None))?;
        }

        Ok(())
    }
}
