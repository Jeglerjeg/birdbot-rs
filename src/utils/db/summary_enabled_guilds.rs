use crate::Error;
use crate::models::summary_enabled_guilds::{NewSummaryEnabledGuild, SummaryEnabledGuild};
use crate::schema::summary_enabled_guilds;
use diesel::insert_into;
use diesel::prelude::{ExpressionMethods, QueryDsl, QueryResult};
use diesel_async::{AsyncPgConnection, RunQueryDsl};

pub async fn create(
    db: &mut AsyncPgConnection,
    item: &NewSummaryEnabledGuild,
) -> Result<SummaryEnabledGuild, Error> {
    Ok(insert_into(summary_enabled_guilds::table)
        .values(item)
        .on_conflict(summary_enabled_guilds::id)
        .do_update()
        .set(item)
        .get_result(db)
        .await?)
}

pub async fn read(
    db: &mut AsyncPgConnection,
    param_guild_id: i64,
) -> QueryResult<SummaryEnabledGuild> {
    summary_enabled_guilds::table
        .filter(summary_enabled_guilds::guild_id.eq(param_guild_id))
        .first(db)
        .await
}

pub async fn update(
    db: &mut AsyncPgConnection,
    param_id: i64,
    item: &NewSummaryEnabledGuild,
) -> QueryResult<SummaryEnabledGuild> {
    diesel::update(summary_enabled_guilds::table.filter(summary_enabled_guilds::id.eq(param_id)))
        .set(item)
        .get_result(db)
        .await
}

pub async fn delete_channel(
    db: &mut AsyncPgConnection,
    param_guild_id: i64,
    values: &NewSummaryEnabledGuild,
) -> Result<usize, Error> {
    Ok(diesel::update(
        summary_enabled_guilds::table.filter(summary_enabled_guilds::guild_id.eq(param_guild_id)),
    )
    .set(values)
    .execute(db)
    .await?)
}
