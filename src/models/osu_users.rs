use crate::schema::osu_users;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Queryable, Identifiable)]
#[diesel(table_name=osu_users, primary_key(id))]
pub struct OsuUser {
    pub id: i64,
    pub username: String,
    pub avatar_url: String,
    pub country_code: String,
    pub mode: String,
    pub pp: f64,
    pub accuracy: f64,
    pub country_rank: i32,
    pub global_rank: i32,
    pub max_combo: i32,
    pub ranked_score: i64,
    pub ticks: i32,
    pub time_cached: chrono::DateTime<chrono::Utc>,
    pub min_pp: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone, Insertable, AsChangeset)]
#[diesel(table_name=osu_users)]
pub struct NewOsuUser {
    pub id: i64,
    pub username: String,
    pub avatar_url: String,
    pub country_code: String,
    pub mode: String,
    pub pp: f64,
    pub accuracy: f64,
    pub country_rank: i32,
    pub global_rank: i32,
    pub max_combo: i32,
    pub ticks: i32,
    pub ranked_score: i64,
    pub time_cached: chrono::DateTime<chrono::Utc>,
    pub min_pp: f64,
}

impl NewOsuUser {
    pub fn add_ticks(&mut self, ticks: i32) {
        self.ticks = ticks;
    }
}
