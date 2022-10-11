use crate::models::osu_guild_channels::{NewOsuGuildChannel, OsuGuildChannel};
use crate::Error;
use diesel::prelude::*;
use diesel::{insert_into, QueryResult, RunQueryDsl};

pub fn create(item: &NewOsuGuildChannel) -> Result<(), Error> {
    use crate::schema::osu_guild_channels::dsl::{guild_id, osu_guild_channels};
    let db = &mut crate::utils::db::establish_connection::establish_connection();

    insert_into(osu_guild_channels)
        .values(item)
        .on_conflict(guild_id)
        .do_update()
        .set(item)
        .execute(db)?;
    Ok(())
}

pub fn read(param_guild_id: i64) -> QueryResult<OsuGuildChannel> {
    use crate::schema::osu_guild_channels::dsl::{guild_id, osu_guild_channels};
    let db = &mut crate::utils::db::establish_connection::establish_connection();

    osu_guild_channels
        .filter(guild_id.eq(param_guild_id))
        .first::<OsuGuildChannel>(db)
}

pub fn delete(param_guild_id: i64) -> Result<usize, Error> {
    use crate::schema::osu_guild_channels::dsl::{guild_id, osu_guild_channels};
    let db = &mut crate::utils::db::establish_connection::establish_connection();

    Ok(diesel::delete(osu_guild_channels.filter(guild_id.eq(param_guild_id))).execute(db)?)
}
