use crate::schema::beatmapsets;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Queryable, Identifiable)]
#[diesel(table_name=beatmapsets, primary_key(id))]
pub struct Beatmapset {
    pub id: i64,
    pub artist: String,
    pub bpm: f64,
    pub list_cover: String,
    pub cover: String,
    pub creator: String,
    pub play_count: i64,
    pub source: String,
    pub status: String,
    pub title: String,
    pub user_id: i64,
    pub time_cached: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone, Insertable, AsChangeset)]
#[diesel(table_name=beatmapsets)]
pub struct NewBeatmapset {
    pub id: i64,
    pub artist: String,
    pub bpm: f64,
    pub list_cover: String,
    pub cover: String,
    pub creator: String,
    pub play_count: i64,
    pub source: String,
    pub status: String,
    pub title: String,
    pub user_id: i64,
}
