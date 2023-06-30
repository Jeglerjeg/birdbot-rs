use crate::schema::osu_guild_channels;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Queryable, Identifiable)]
#[diesel(table_name=osu_guild_channels, primary_key(guild_id))]
pub struct OsuGuildChannel {
    pub guild_id: i64,
    pub score_channel: Option<Vec<Option<i64>>>,
    pub map_channel: Option<Vec<Option<i64>>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Insertable, AsChangeset)]
#[diesel(table_name=osu_guild_channels)]
pub struct NewOsuGuildChannel {
    pub guild_id: i64,
    pub score_channel: Option<Vec<Option<i64>>>,
    pub map_channel: Option<Vec<Option<i64>>>,
}
