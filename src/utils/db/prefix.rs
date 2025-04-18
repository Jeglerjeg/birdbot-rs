use crate::models::prefix::{NewPrefix, Prefix};
use crate::schema::prefix;
use crate::{Data, Error, PartialContext};
use dashmap::DashMap;
use diesel::prelude::QueryDsl;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use poise::serenity_prelude::GuildId;
use std::borrow::Cow;
use std::env;
use std::sync::OnceLock;

static DEFAULT_PREFIX: OnceLock<String> = OnceLock::new();

pub struct GuildPrefix {
    pub guild_prefix: DashMap<GuildId, String>,
}

static GUILD_PREFIX: OnceLock<GuildPrefix> = OnceLock::new();

pub async fn add_guild_prefix(
    db: &mut AsyncPgConnection,
    guild_id: i64,
    guild_prefix: String,
) -> Result<(), Error> {
    let new_prefix = NewPrefix {
        guild_id,
        guild_prefix: guild_prefix.clone(),
    };

    GUILD_PREFIX
        .get_or_init(|| GuildPrefix {
            guild_prefix: DashMap::new(),
        })
        .guild_prefix
        .insert(GuildId::from(u64::try_from(guild_id)?), guild_prefix);

    diesel::insert_into(prefix::table)
        .values(&new_prefix)
        .on_conflict(prefix::guild_id)
        .do_update()
        .set(&new_prefix)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn get_guild_prefix(ctx: PartialContext<'_>) -> Result<Option<Cow<'static, str>>, Error> {
    let Some(guild_id) = ctx.guild_id else {
        return Ok(Some(Cow::from(DEFAULT_PREFIX.get_or_init(|| {
            env::var("PREFIX").unwrap_or_else(|_| String::from(">"))
        }))));
    };

    let guild_prefix = if let Some(guild_prefix) = GUILD_PREFIX
        .get_or_init(|| GuildPrefix {
            guild_prefix: DashMap::new(),
        })
        .guild_prefix
        .get(&guild_id)
    {
        guild_prefix.clone()
    } else {
        let prefix = match prefix::table
            .find(i64::try_from(guild_id.get())?)
            .first::<Prefix>(
                &mut ctx
                    .framework
                    .serenity_context
                    .data::<Data>()
                    .db_pool
                    .get()
                    .await?,
            )
            .await
        {
            Ok(prefix) => prefix.guild_prefix,
            _ => DEFAULT_PREFIX
                .get_or_init(|| env::var("PREFIX").unwrap_or_else(|_| String::from(">")))
                .to_owned(),
        };

        GUILD_PREFIX
            .get_or_init(|| GuildPrefix {
                guild_prefix: DashMap::new(),
            })
            .guild_prefix
            .insert(guild_id, prefix.clone());
        prefix
    };

    Ok(Some(Cow::from(guild_prefix)))
}
