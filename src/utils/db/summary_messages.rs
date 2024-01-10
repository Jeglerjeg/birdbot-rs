use crate::models::summary_messages::NewDbSummaryMessage;
use crate::schema::summary_messages;
use crate::Error;
use diesel::dsl::count;
use diesel::insert_into;
use diesel::prelude::{ExpressionMethods, QueryDsl};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use diesel_full_text_search::{plainto_tsquery, TsVectorExtensions};
use par_stream::{ParParams, ParStreamExt};
use tokio_stream::StreamExt;

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
) -> Result<Vec<Vec<String>>, Error> {
    let mut query = summary_messages::table
        .filter(summary_messages::channel_id.eq_any(channel_ids))
        .into_boxed();
    if let Some(phrase) = phrase {
        let ts_query = plainto_tsquery(phrase);
        query = query.filter(summary_messages::ts.matches(ts_query));
    }
    if !include_bots {
        query = query.filter(summary_messages::is_bot.eq(false));
    }
    if !author_ids.is_empty() {
        query = query.filter(summary_messages::author_id.eq_any(author_ids));
    }
    let messages = query
        .select(summary_messages::content)
        .load_stream::<String>(db)
        .await?
        .par_then_unordered(
            ParParams {
                num_workers: num_cpus::get(),
                buf_size: Some(512),
            },
            |value| async move {
                value
                    .expect("Couldn't get message value from summary db")
                    .split_whitespace()
                    .map(std::borrow::ToOwned::to_owned)
                    .collect::<Vec<_>>()
            },
        )
        .collect::<Vec<_>>()
        .await;
    Ok(messages)
}

pub async fn count_entries(db: &mut AsyncPgConnection, channel_id: i64) -> Result<i64, Error> {
    Ok(summary_messages::table
        .filter(summary_messages::channel_id.eq(channel_id))
        .select(count(summary_messages::id))
        .get_result(db)
        .await?)
}

pub async fn delete(db: &mut AsyncPgConnection, param_channel_id: i64) -> Result<usize, Error> {
    Ok(diesel::delete(
        summary_messages::table.filter(summary_messages::channel_id.eq(param_channel_id)),
    )
    .execute(db)
    .await?)
}
