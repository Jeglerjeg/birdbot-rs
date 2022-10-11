use crate::schema::prefix;
use diesel::prelude::*;

#[derive(Queryable)]
pub struct Prefix {
    pub guild_id: i64,
    pub guild_prefix: String,
}

#[derive(Insertable, AsChangeset)]
#[diesel(table_name = prefix)]
pub struct NewPrefix<'a> {
    pub guild_id: &'a i64,
    pub guild_prefix: &'a str,
}
