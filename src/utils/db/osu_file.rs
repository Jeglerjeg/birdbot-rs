use crate::models::osu_files::{NewOsuFile, OsuFile};
use crate::schema::osu_files;
use crate::schema::osu_files::id;
use crate::Error;
use diesel::prelude::QueryDsl;
use diesel::upsert::excluded;
use diesel::{insert_into, ExpressionMethods};
use diesel_async::{AsyncPgConnection, RunQueryDsl};

pub async fn create(
    db: &mut AsyncPgConnection,
    osu_files: Vec<(i64, Vec<u8>)>,
) -> Result<(), Error> {
    let mut items = Vec::new();

    for osu_file in osu_files {
        items.push(NewOsuFile {
            id: osu_file.0,
            file: osu_file.1,
        });
    }

    insert_into(osu_files::table)
        .values(items)
        .on_conflict(osu_files::id)
        .do_update()
        .set((
            osu_files::id.eq(excluded(osu_files::id)),
            osu_files::file.eq(excluded(osu_files::file)),
        ))
        .execute(db)
        .await?;

    Ok(())
}

pub async fn get_files(db: &mut AsyncPgConnection, ids: &[i64]) -> Result<Vec<OsuFile>, Error> {
    let osu_file = osu_files::table
        .filter(id.eq_any(ids))
        .load::<OsuFile>(db)
        .await?;

    Ok(osu_file)
}

pub async fn update(db: &mut AsyncPgConnection, param_id: i64, file: Vec<u8>) -> Result<(), Error> {
    let item = NewOsuFile { id: param_id, file };

    diesel::update(osu_files::table.find(param_id))
        .set(item)
        .execute(db)
        .await?;

    Ok(())
}
