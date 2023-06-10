use crate::models::osu_users::{NewOsuUser, OsuUser};
use crate::Error;
use chrono::Utc;
use diesel::insert_into;
use diesel::prelude::{ExpressionMethods, QueryDsl, QueryResult};
use diesel_async::{AsyncPgConnection, RunQueryDsl};

pub fn rosu_user_to_db(
    user: rosu_v2::prelude::User,
    ticks: Option<i32>,
) -> Result<NewOsuUser, Error> {
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
        country_rank: statistic.country_rank.unwrap_or(0) as i32,
        global_rank: statistic.global_rank.unwrap_or(0) as i32,
        max_combo: statistic.max_combo as i32,
        ranked_score: statistic.ranked_score as i64,
        ticks: ticks.unwrap_or(0),
        time_cached: Utc::now(),
    })
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
