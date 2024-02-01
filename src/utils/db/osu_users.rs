use crate::models::osu_users::{NewOsuUser, OsuUser};
use crate::schema::osu_users;
use crate::Error;
use chrono::Utc;
use diesel::insert_into;
use diesel::prelude::{ExpressionMethods, QueryDsl, QueryResult};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use rosu_v2::prelude::UserExtended;

impl TryFrom<UserExtended> for NewOsuUser {
    type Error = Error;

    fn try_from(user: UserExtended) -> Result<Self, Self::Error> {
        let statistic = user
            .statistics
            .ok_or("Failed to get user statistic in rosu_user_to_db function")?;
        Ok(NewOsuUser {
            id: i64::from(user.user_id),
            username: user.username.to_string(),
            avatar_url: user.avatar_url,
            country_code: user.country_code.into_string(),
            mode: user.mode.to_string(),
            pp: f64::from(statistic.pp),
            accuracy: f64::from(statistic.accuracy),
            country_rank: i32::try_from(statistic.country_rank.unwrap_or(0))?,
            global_rank: i32::try_from(statistic.global_rank.unwrap_or(0))?,
            max_combo: i32::try_from(statistic.max_combo)?,
            ticks: 0,
            ranked_score: i64::try_from(statistic.ranked_score)?,
            time_cached: Utc::now(),
        })
    }
}

pub async fn create(db: &mut AsyncPgConnection, item: &NewOsuUser) -> Result<OsuUser, Error> {
    use crate::schema::osu_users::dsl::{id, osu_users};

    let user = insert_into(osu_users)
        .values(item)
        .on_conflict(id)
        .do_update()
        .set(item)
        .get_result(db)
        .await?;

    Ok(user)
}

pub async fn update_ticks(
    db: &mut AsyncPgConnection,
    profile_id: i64,
    ticks: i32,
) -> Result<(), Error> {
    diesel::update(osu_users::table.find(profile_id))
        .set(osu_users::ticks.eq(ticks))
        .execute(db)
        .await?;

    Ok(())
}

pub async fn read(db: &mut AsyncPgConnection, param_id: i64) -> QueryResult<OsuUser> {
    use crate::schema::osu_users::dsl::{id, osu_users};

    osu_users.filter(id.eq(param_id)).first::<OsuUser>(db).await
}

pub async fn delete(db: &mut AsyncPgConnection, param_id: i64) -> Result<(), Error> {
    use crate::schema::osu_users::dsl::{id, osu_users};

    diesel::delete(osu_users.filter(id.eq(param_id)))
        .execute(db)
        .await?;

    Ok(())
}

pub async fn get_all(db: &mut AsyncPgConnection) -> Result<Vec<OsuUser>, Error> {
    use crate::schema::osu_users::dsl::osu_users;

    Ok(osu_users.load::<OsuUser>(db).await?)
}
