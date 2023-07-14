use crate::models::osu_guild_channels::{NewOsuGuildChannel, OsuGuildChannel};
use crate::schema::osu_guild_channels;
use crate::Error;
use diesel::dsl::count;
use diesel::insert_into;
use diesel::prelude::{ExpressionMethods, QueryDsl, QueryResult};
use diesel_async::{AsyncPgConnection, RunQueryDsl};

pub async fn create(db: &mut AsyncPgConnection, item: &NewOsuGuildChannel) -> Result<(), Error> {
    insert_into(osu_guild_channels::table)
        .values(item)
        .on_conflict(osu_guild_channels::guild_id)
        .do_update()
        .set(item)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn count_entries(db: &mut AsyncPgConnection) -> Result<i64, Error> {
    Ok(osu_guild_channels::table
        .select(count(osu_guild_channels::guild_id))
        .get_result(db)
        .await?)
}

pub async fn read(db: &mut AsyncPgConnection, param_guild_id: i64) -> QueryResult<OsuGuildChannel> {
    osu_guild_channels::table
        .filter(osu_guild_channels::guild_id.eq(param_guild_id))
        .first::<OsuGuildChannel>(db)
        .await
}

pub async fn delete(db: &mut AsyncPgConnection, param_guild_id: i64) -> Result<usize, Error> {
    Ok(diesel::delete(
        osu_guild_channels::table.filter(osu_guild_channels::guild_id.eq(param_guild_id)),
    )
    .execute(db)
    .await?)
}
