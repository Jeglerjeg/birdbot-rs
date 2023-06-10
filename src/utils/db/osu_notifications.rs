use crate::models::osu_notifications::{NewOsuNotification, OsuNotification};
use diesel::insert_into;
use diesel::prelude::{ExpressionMethods, QueryDsl, QueryResult};
use diesel_async::{AsyncPgConnection, RunQueryDsl};

pub async fn create(
    db: &mut AsyncPgConnection,
    item: &NewOsuNotification,
) -> QueryResult<OsuNotification> {
    use crate::schema::osu_notifications::dsl::{id, osu_notifications};

    insert_into(osu_notifications)
        .values(item)
        .on_conflict(id)
        .do_update()
        .set(item)
        .get_result::<OsuNotification>(db)
        .await
}

pub async fn read(db: &mut AsyncPgConnection, param_id: i64) -> QueryResult<OsuNotification> {
    use crate::schema::osu_notifications::dsl::{id, osu_notifications};

    osu_notifications
        .filter(id.eq(param_id))
        .first::<OsuNotification>(db)
        .await
}

pub async fn update(
    db: &mut AsyncPgConnection,
    param_id: i64,
    item: &NewOsuNotification,
) -> QueryResult<OsuNotification> {
    use crate::schema::osu_notifications::dsl::{id, osu_notifications};

    diesel::update(osu_notifications.filter(id.eq(param_id)))
        .set(item)
        .get_result(db)
        .await
}

pub async fn delete(db: &mut AsyncPgConnection, param_id: i64) -> QueryResult<usize> {
    use crate::schema::osu_notifications::dsl::{id, osu_notifications};

    diesel::delete(osu_notifications.filter(id.eq(param_id)))
        .execute(db)
        .await
}
