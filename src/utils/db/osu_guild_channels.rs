use crate::models::osu_guild_channels::{NewOsuGuildChannel, OsuGuildChannel};
use crate::Error;
use diesel::insert_into;
use diesel::prelude::{ExpressionMethods, QueryDsl, QueryResult};
use diesel_async::{AsyncPgConnection, RunQueryDsl};

pub async fn create(db: &mut AsyncPgConnection, item: &NewOsuGuildChannel) -> Result<(), Error> {
    use crate::schema::osu_guild_channels::dsl::{guild_id, osu_guild_channels};

    insert_into(osu_guild_channels)
        .values(item)
        .on_conflict(guild_id)
        .do_update()
        .set(item)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn read(db: &mut AsyncPgConnection, param_guild_id: i64) -> QueryResult<OsuGuildChannel> {
    use crate::schema::osu_guild_channels::dsl::{guild_id, osu_guild_channels};

    osu_guild_channels
        .filter(guild_id.eq(param_guild_id))
        .first::<OsuGuildChannel>(db)
        .await
}

pub async fn delete(db: &mut AsyncPgConnection, param_guild_id: i64) -> Result<usize, Error> {
    use crate::schema::osu_guild_channels::dsl::{guild_id, osu_guild_channels};

    Ok(
        diesel::delete(osu_guild_channels.filter(guild_id.eq(param_guild_id)))
            .execute(db)
            .await?,
    )
}
