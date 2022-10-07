/* This file is generated and managed by dsync */

use crate::schema::beatmaps;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Serialize, Deserialize, Clone, Queryable, Insertable, AsChangeset, Identifiable,
)]
#[diesel(table_name=beatmaps, primary_key(id))]
pub struct Beatmap {
    pub id: i64,
    pub ar: f32,
    pub beatmapset_id: i64,
    pub checksum: String,
    pub max_combo: i32,
    pub bpm: f32,
    pub convert: bool,
    pub count_circles: i32,
    pub count_sliders: i32,
    pub count_spinners: i32,
    pub cs: f32,
    pub difficulty_rating: f32,
    pub drain: i32,
    pub mode: String,
    pub passcount: i32,
    pub playcount: i32,
    pub status: String,
    pub total_length: i32,
    pub user_id: i64,
    pub version: String,
    pub time_cached: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone, Queryable, Insertable, AsChangeset)]
#[diesel(table_name=beatmaps)]
pub struct NewBeatmap {
    pub id: i64,
    pub ar: f32,
    pub beatmapset_id: i64,
    pub checksum: String,
    pub max_combo: i32,
    pub bpm: f32,
    pub convert: bool,
    pub count_circles: i32,
    pub count_sliders: i32,
    pub count_spinners: i32,
    pub cs: f32,
    pub difficulty_rating: f32,
    pub drain: i32,
    pub mode: String,
    pub passcount: i32,
    pub playcount: i32,
    pub status: String,
    pub total_length: i32,
    pub user_id: i64,
    pub version: String,
}
