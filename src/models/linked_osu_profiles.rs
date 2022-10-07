use crate::schema::linked_osu_profiles;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Serialize, Deserialize, Clone, Queryable, Insertable, AsChangeset, Identifiable,
)]
#[diesel(table_name=linked_osu_profiles, primary_key(id))]
pub struct LinkedOsuProfile {
    pub id: i64,
    pub osu_id: i64,
    pub home_guild: i64,
    pub mode: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Queryable, Insertable, AsChangeset)]
#[diesel(table_name=linked_osu_profiles)]
pub struct NewLinkedOsuProfile {
    pub id: i64,
    pub osu_id: i64,
    pub home_guild: i64,
    pub mode: String,
}
