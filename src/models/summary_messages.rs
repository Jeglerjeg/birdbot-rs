use crate::schema::summary_messages;
use diesel::{AsChangeset, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Queryable)]
#[diesel(table_name=summary_messages)]
pub struct DbSummaryMessage {
    pub id: i64,
    pub content: String,
    pub discord_id: i64,
    pub author_id: i64,
    pub channel_id: i64,
    pub is_bot: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Insertable, AsChangeset)]
#[diesel(table_name=summary_messages, primary_key(id))]
pub struct NewDbSummaryMessage {
    pub content: String,
    pub discord_id: i64,
    pub author_id: i64,
    pub channel_id: i64,
    pub is_bot: bool,
}
