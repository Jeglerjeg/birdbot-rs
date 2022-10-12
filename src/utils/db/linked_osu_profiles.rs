use crate::models::linked_osu_profiles::{LinkedOsuProfile, NewLinkedOsuProfile};
use crate::Error;
use diesel::prelude::*;
use diesel::{insert_into, QueryResult, RunQueryDsl};

pub fn create(db: &mut PgConnection, item: &NewLinkedOsuProfile) -> Result<(), Error> {
    use crate::schema::linked_osu_profiles::dsl::{id, linked_osu_profiles};

    insert_into(linked_osu_profiles)
        .values(item)
        .on_conflict(id)
        .do_update()
        .set(item)
        .execute(db)?;

    Ok(())
}

pub fn read(db: &mut PgConnection, param_id: i64) -> QueryResult<LinkedOsuProfile> {
    use crate::schema::linked_osu_profiles::dsl::{id, linked_osu_profiles};

    linked_osu_profiles
        .filter(id.eq(param_id))
        .first::<LinkedOsuProfile>(db)
}

pub fn get_all(db: &mut PgConnection) -> Result<Vec<LinkedOsuProfile>, Error> {
    use crate::schema::linked_osu_profiles::dsl::linked_osu_profiles;

    Ok(linked_osu_profiles.load::<LinkedOsuProfile>(db)?)
}

pub fn update(
    db: &mut PgConnection,
    param_id: i64,
    item: &NewLinkedOsuProfile,
) -> Result<(), Error> {
    use crate::schema::linked_osu_profiles::dsl::{id, linked_osu_profiles};

    diesel::update(linked_osu_profiles.filter(id.eq(param_id)))
        .set(item)
        .execute(db)?;

    Ok(())
}

pub fn delete(db: &mut PgConnection, param_id: i64) -> QueryResult<usize> {
    use crate::schema::linked_osu_profiles::dsl::{id, linked_osu_profiles};

    diesel::delete(linked_osu_profiles.filter(id.eq(param_id))).execute(db)
}
