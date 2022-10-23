use crate::schema::osu_notifications;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Queryable, Identifiable)]
#[diesel(table_name=osu_notifications, primary_key(id))]
pub struct OsuNotification {
    pub id: i64,
    pub last_pp: chrono::DateTime<chrono::Utc>,
    pub last_event: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Insertable, AsChangeset)]
#[diesel(table_name=osu_notifications)]
pub struct NewOsuNotification {
    pub id: i64,
    pub last_pp: chrono::DateTime<chrono::Utc>,
    pub last_event: chrono::DateTime<chrono::Utc>,
}
