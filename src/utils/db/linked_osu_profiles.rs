use crate::models::linked_osu_profiles::{LinkedOsuProfile, NewLinkedOsuProfile};
use crate::Error;
use diesel::prelude::*;
use diesel::{insert_into, QueryResult, RunQueryDsl};

pub fn create(item: &NewLinkedOsuProfile) -> Result<(), Error> {
    use crate::schema::linked_osu_profiles::dsl::{id, linked_osu_profiles};
    let db = &mut crate::utils::db::establish_connection::establish_connection();

    insert_into(linked_osu_profiles)
        .values(item)
        .on_conflict(id)
        .do_update()
        .set(item)
        .execute(db)?;

    Ok(())
}

pub fn read(param_id: i64) -> QueryResult<LinkedOsuProfile> {
    use crate::schema::linked_osu_profiles::dsl::{id, linked_osu_profiles};
    let db = &mut crate::utils::db::establish_connection::establish_connection();

    linked_osu_profiles
        .filter(id.eq(param_id))
        .first::<LinkedOsuProfile>(db)
}

pub fn get_all() -> Result<Vec<LinkedOsuProfile>, Error> {
    use crate::schema::linked_osu_profiles::dsl::linked_osu_profiles;
    let db = &mut crate::utils::db::establish_connection::establish_connection();

    Ok(linked_osu_profiles.load::<LinkedOsuProfile>(db)?)
}

pub fn update(param_id: i64, item: &NewLinkedOsuProfile) {
    use crate::schema::linked_osu_profiles::dsl::{id, linked_osu_profiles};
    let db = &mut crate::utils::db::establish_connection::establish_connection();

    diesel::update(linked_osu_profiles.filter(id.eq(param_id)))
        .set(item)
        .execute(db)
        .expect("Couldn't update osu profile");
}

pub fn delete(param_id: i64) -> QueryResult<usize> {
    use crate::schema::linked_osu_profiles::dsl::{id, linked_osu_profiles};
    let db = &mut crate::utils::db::establish_connection::establish_connection();

    diesel::delete(linked_osu_profiles.filter(id.eq(param_id))).execute(db)
}
