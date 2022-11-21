use crate::schema::prefix;
use diesel::{AsChangeset, Insertable, Queryable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Queryable)]
pub struct Prefix {
    pub guild_id: i64,
    pub guild_prefix: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Insertable, AsChangeset)]
#[diesel(table_name = prefix)]
pub struct NewPrefix {
    pub guild_id: i64,
    pub guild_prefix: String,
}
