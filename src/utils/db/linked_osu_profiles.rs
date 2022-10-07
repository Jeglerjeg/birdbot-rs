use crate::models::linked_osu_profiles::{LinkedOsuProfile, NewLinkedOsuProfile};
use diesel::prelude::*;
use diesel::{replace_into, QueryResult, RunQueryDsl};

pub fn create(item: &NewLinkedOsuProfile) {
    use crate::schema::linked_osu_profiles::dsl::linked_osu_profiles;
    let db = &mut crate::utils::db::establish_connection::establish_connection();

    replace_into(linked_osu_profiles)
        .values(item)
        .execute(db)
        .expect("Couldn't insert osu profile");
}

pub fn read(param_id: i64) -> QueryResult<LinkedOsuProfile> {
    use crate::schema::linked_osu_profiles::dsl::{id, linked_osu_profiles};
    let db = &mut crate::utils::db::establish_connection::establish_connection();

    linked_osu_profiles
        .filter(id.eq(param_id))
        .first::<LinkedOsuProfile>(db)
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
