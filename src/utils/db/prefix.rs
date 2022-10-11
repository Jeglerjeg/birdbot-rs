use crate::models::prefix::{NewPrefix, Prefix};
use crate::schema::prefix;
use crate::{Error, PartialContext};
use diesel::prelude::*;
use lazy_static::lazy_static;
use std::env;

lazy_static! {
    static ref DEFAULT_PREFIX: String = env::var("PREFIX").unwrap_or_else(|_| String::from(">"));
}

pub fn add_guild_prefix(db: &mut PgConnection, guild_id: i64, prefix: &str) {
    let new_prefix = NewPrefix {
        guild_id: &guild_id,
        guild_prefix: prefix,
    };

    diesel::insert_into(prefix::table)
        .values(&new_prefix)
        .on_conflict(prefix::guild_id)
        .do_update()
        .set(&new_prefix)
        .execute(db)
        .expect("Failed to insert prefix");
}

pub async fn get_guild_prefix(ctx: PartialContext<'_>) -> Result<Option<String>, Error> {
    let guild_id = match ctx.guild_id {
        Some(guild) => guild.0 as i64,
        _ => return Ok(Some(DEFAULT_PREFIX.clone())),
    };

    let connection = &mut ctx.data.db_pool.get().unwrap();
    let db_prefix = prefix::table
        .find(guild_id)
        .limit(1)
        .load::<Prefix>(connection)
        .expect("Error loading guild prefix");

    if db_prefix.is_empty() {
        Ok(Some(DEFAULT_PREFIX.clone()))
    } else {
        Ok(Some(db_prefix[0].guild_prefix.clone()))
    }
}
