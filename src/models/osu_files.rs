use crate::models::beatmaps::Beatmap;
use crate::schema::osu_files;
use diesel::{AsChangeset, Associations, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Serialize, Deserialize, Clone, Queryable, Associations, Identifiable, Selectable,
)]
#[diesel(belongs_to(Beatmap, foreign_key = id))]
#[diesel(table_name=osu_files, primary_key(id))]
pub struct OsuFile {
    pub id: i64,
    pub file: Vec<u8>,
}

#[derive(
    Debug,
    Serialize,
    Deserialize,
    Clone,
    Queryable,
    Associations,
    Identifiable,
    Selectable,
    AsChangeset,
    Insertable,
)]
#[diesel(belongs_to(Beatmap, foreign_key = id))]
#[diesel(table_name=osu_files, primary_key(id))]
pub struct NewOsuFile {
    pub id: i64,
    pub file: Vec<u8>,
}
