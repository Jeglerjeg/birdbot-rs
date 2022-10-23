use crate::models::osu_notifications::{NewOsuNotification, OsuNotification};
use diesel::prelude::*;
use diesel::{insert_into, QueryResult, RunQueryDsl};

pub fn create(db: &mut PgConnection, item: &NewOsuNotification) -> QueryResult<OsuNotification> {
    use crate::schema::osu_notifications::dsl::{id, osu_notifications};

    insert_into(osu_notifications)
        .values(item)
        .on_conflict(id)
        .do_update()
        .set(item)
        .get_result::<OsuNotification>(db)
}

pub fn read(db: &mut PgConnection, param_id: i64) -> QueryResult<OsuNotification> {
    use crate::schema::osu_notifications::dsl::{id, osu_notifications};

    osu_notifications
        .filter(id.eq(param_id))
        .first::<OsuNotification>(db)
}

pub fn update(
    db: &mut PgConnection,
    param_id: i64,
    item: &NewOsuNotification,
) -> QueryResult<OsuNotification> {
    use crate::schema::osu_notifications::dsl::{id, osu_notifications};

    diesel::update(osu_notifications.filter(id.eq(param_id)))
        .set(item)
        .get_result(db)
}

pub fn delete(db: &mut PgConnection, param_id: i64) -> QueryResult<usize> {
    use crate::schema::osu_notifications::dsl::{id, osu_notifications};

    diesel::delete(osu_notifications.filter(id.eq(param_id))).execute(db)
}
