use crate::models::prefix::{NewPrefix, Prefix};
use crate::schema::prefix;
use crate::{Error, PartialContext};
use dashmap::DashMap;
use diesel::prelude::*;
use lazy_static::lazy_static;
use poise::serenity_prelude::GuildId;
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

pub fn add_guild_prefix(
    db: &mut PgConnection,
    guild_id: i64,
    guild_prefix: String,
) -> Result<(), Error> {
    let new_prefix = NewPrefix {
        guild_id,
        guild_prefix: guild_prefix.clone(),
    };

    GUILD_PREFIX
        .guild_prefix
        .insert(GuildId::from(guild_id as u64), guild_prefix);

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
        Some(guild) => guild,
        _ => return Ok(Some(DEFAULT_PREFIX.clone())),
    };

    Ok(Some(
        GUILD_PREFIX
            .guild_prefix
            .entry(guild_id)
            .or_insert(
                match prefix::table
                    .find(guild_id.get() as i64)
                    .limit(1)
                    .load::<Prefix>(&mut ctx.data.db_pool.get()?)?
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
