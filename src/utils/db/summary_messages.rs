use crate::models::summary_messages::NewDbSummaryMessage;
use crate::schema::summary_messages;
use crate::Error;
use diesel::dsl::count;
use diesel::insert_into;
use diesel::prelude::{ExpressionMethods, PgTextExpressionMethods, QueryDsl};
use diesel_async::{AsyncPgConnection, RunQueryDsl};

pub async fn create(
    db: &mut AsyncPgConnection,
    item: &Vec<NewDbSummaryMessage>,
) -> Result<(), Error> {
    insert_into(summary_messages::table)
        .values(item)
        .execute(db)
        .await?;

    Ok(())
}

pub async fn read(
    db: &mut AsyncPgConnection,
    include_bots: bool,
    phrase: Option<String>,
    author_ids: Vec<i64>,
    channel_ids: Vec<i64>,
) -> Result<Vec<String>, Error> {
    let mut query = summary_messages::table
        .filter(summary_messages::channel_id.eq_any(channel_ids))
        .into_boxed();
    if let Some(phrase) = phrase {
        query = query.filter(summary_messages::content.ilike(format!("%{}%", phrase)));
    }
    if !include_bots {
        query = query.filter(summary_messages::is_bot.eq(false))
    }
    if !author_ids.is_empty() {
        query = query.filter(summary_messages::author_id.eq_any(author_ids))
    }
    Ok(query
        .select(summary_messages::content)
        .load::<String>(db)
        .await?)
}

pub async fn count_entries(db: &mut AsyncPgConnection, channel_id: i64) -> Result<i64, Error> {
    Ok(summary_messages::table
        .filter(summary_messages::channel_id.eq(channel_id))
        .select(count(summary_messages::id))
        .get_result(db)
        .await?)
}

pub async fn delete(db: &mut AsyncPgConnection, param_guild_id: i64) -> Result<usize, Error> {
    Ok(diesel::delete(
        summary_messages::table.filter(summary_messages::guild_id.eq(param_guild_id)),
    )
    .execute(db)
    .await?)
}
