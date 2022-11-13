use crate::models::prefix::{NewPrefix, Prefix};
use crate::schema::prefix;
use crate::serenity_prelude::GuildId;
use crate::{Error, PartialContext};
use dashmap::DashMap;
use diesel::prelude::*;
use lazy_static::lazy_static;
use std::env;

lazy_static! {
    static ref DEFAULT_PREFIX: String = env::var("PREFIX").unwrap_or_else(|_| String::from(">"));
}

pub struct GuildPrefix {
    pub guild_prefix: DashMap<GuildId, String>,
}

lazy_static! {
    static ref GUILD_PREFIX: GuildPrefix = GuildPrefix {
        guild_prefix: DashMap::new(),
    };
}

pub fn add_guild_prefix(db: &mut PgConnection, guild_id: i64, prefix: &str) -> Result<(), Error> {
    let new_prefix = NewPrefix {
        guild_id: &guild_id,
        guild_prefix: prefix,
    };

    GUILD_PREFIX
        .guild_prefix
        .insert(GuildId::from(guild_id as u64), prefix.into());

    diesel::insert_into(prefix::table)
        .values(&new_prefix)
        .on_conflict(prefix::guild_id)
        .do_update()
        .set(&new_prefix)
        .execute(db)?;

    Ok(())
}

pub async fn get_guild_prefix(ctx: PartialContext<'_>) -> Result<Option<String>, Error> {
    let guild_id = match ctx.guild_id {
        Some(guild) => guild.0.get() as i64,
        _ => return Ok(Some(DEFAULT_PREFIX.clone())),
    };

    let connection = &mut ctx.data.db_pool.get().unwrap();

    Ok(Some(
        GUILD_PREFIX
            .guild_prefix
            .entry(ctx.guild_id.unwrap())
            .or_insert(
                match prefix::table
                    .find(guild_id)
                    .limit(1)
                    .load::<Prefix>(connection)?
                    .get(0)
                {
                    Some(prefix) => prefix.guild_prefix.clone(),
                    _ => DEFAULT_PREFIX.clone(),
                },
            )
            .value()
            .to_string(),
    ))
}
