use crate::schema::summary_enabled_guilds;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Queryable, Identifiable)]
#[diesel(table_name=summary_enabled_guilds)]
pub struct SummaryEnabledGuild {
    pub id: i64,
    pub guild_id: i64,
    pub channel_ids: Vec<Option<i64>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Insertable, AsChangeset)]
#[diesel(table_name=summary_enabled_guilds)]
pub struct NewSummaryEnabledGuild {
    pub guild_id: i64,
    pub channel_ids: Vec<Option<i64>>,
}
