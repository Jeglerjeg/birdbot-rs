use crate::models::osu_users::{NewOsuUser, OsuUser};
use crate::Error;
use chrono::Utc;
use diesel::prelude::*;
use diesel::{insert_into, QueryResult, RunQueryDsl};

pub fn rosu_user_to_db(user: rosu_v2::prelude::User, ticks: Option<i32>) -> NewOsuUser {
    let statistic = user.statistics.unwrap();
    NewOsuUser {
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
    }
}

pub fn create(db: &mut PgConnection, item: &NewOsuUser) -> Result<OsuUser, Error> {
    use crate::schema::osu_users::dsl::{id, osu_users};

    let user = insert_into(osu_users)
        .values(item)
        .on_conflict(id)
        .do_update()
        .set(item)
        .get_result(db)?;

    Ok(user)
}

pub fn read(db: &mut PgConnection, param_id: i64) -> QueryResult<OsuUser> {
    use crate::schema::osu_users::dsl::{id, osu_users};

    osu_users.filter(id.eq(param_id)).first::<OsuUser>(db)
}

pub fn delete(db: &mut PgConnection, param_id: i64) -> Result<(), Error> {
    use crate::schema::osu_users::dsl::{id, osu_users};

    diesel::delete(osu_users.filter(id.eq(param_id))).execute(db)?;

    Ok(())
}
