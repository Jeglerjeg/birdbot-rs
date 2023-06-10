use crate::models::linked_osu_profiles::{LinkedOsuProfile, NewLinkedOsuProfile};
use crate::Error;
use diesel::insert_into;
use diesel::prelude::{ExpressionMethods, QueryDsl, QueryResult};
use diesel_async::{AsyncPgConnection, RunQueryDsl};

pub async fn create(db: &mut AsyncPgConnection, item: &NewLinkedOsuProfile) -> Result<(), Error> {
    use crate::schema::linked_osu_profiles::dsl::{id, linked_osu_profiles};

    insert_into(linked_osu_profiles)
        .values(item)
        .on_conflict(id)
        .do_update()
        .set(item)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn read(db: &mut AsyncPgConnection, param_id: i64) -> QueryResult<LinkedOsuProfile> {
    use crate::schema::linked_osu_profiles::dsl::{id, linked_osu_profiles};

    linked_osu_profiles
        .filter(id.eq(param_id))
        .first::<LinkedOsuProfile>(db)
        .await
}

pub async fn get_all(db: &mut AsyncPgConnection) -> Result<Vec<LinkedOsuProfile>, Error> {
    use crate::schema::linked_osu_profiles::dsl::linked_osu_profiles;

    Ok(linked_osu_profiles.load::<LinkedOsuProfile>(db).await?)
}

pub async fn update(
    db: &mut AsyncPgConnection,
    param_id: i64,
    item: &NewLinkedOsuProfile,
) -> Result<(), Error> {
    use crate::schema::linked_osu_profiles::dsl::{id, linked_osu_profiles};

    diesel::update(linked_osu_profiles.filter(id.eq(param_id)))
        .set(item)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn delete(db: &mut AsyncPgConnection, param_id: i64) -> QueryResult<usize> {
    use crate::schema::linked_osu_profiles::dsl::{id, linked_osu_profiles};

    diesel::delete(linked_osu_profiles.filter(id.eq(param_id)))
        .execute(db)
        .await
}
