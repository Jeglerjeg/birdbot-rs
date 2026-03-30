use crate::Error;
use crate::models::osu_files::NewOsuFile;
use crate::schema::osu_files;
use diesel::dsl::count;
use diesel::prelude::QueryDsl;
use diesel::{ExpressionMethods, insert_into};
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
            osu_files::id.eq(osu_files::id),
            osu_files::file.eq(osu_files::file),
        ))
        .execute(db)
        .await?;

    Ok(())
}

pub async fn count_entries(db: &mut AsyncPgConnection) -> Result<i64, Error> {
    Ok(osu_files::table
        .select(count(osu_files::id))
        .get_result(db)
        .await?)
}

pub async fn delete(db: &mut AsyncPgConnection, param_id: i64) -> Result<(), Error> {
    diesel::delete(osu_files::table.find(param_id))
        .execute(db)
        .await?;

    Ok(())
}
