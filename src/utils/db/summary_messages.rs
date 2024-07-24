use crate::models::summary_messages::NewDbSummaryMessage;
use crate::schema::summary_messages;
use crate::Error;
use diesel::dsl::count;
use diesel::prelude::{ExpressionMethods, QueryDsl};
use diesel::{insert_into, PgTextExpressionMethods};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use markov::Chain;
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

pub async fn construct_chain(
    db: &mut AsyncPgConnection,
    include_bots: bool,
    phrase: Option<String>,
    author_ids: Vec<i64>,
    channel_ids: Vec<i64>,
    n_grams: usize,
) -> Result<Chain<String>, Error> {
    let mut query = summary_messages::table
        .filter(summary_messages::channel_id.eq_any(channel_ids))
        .into_boxed();
    if let Some(phrase) = phrase {
        query = query.filter(summary_messages::content.ilike(format!("%{phrase}%")));
    }
    if !include_bots {
        query = query.filter(summary_messages::is_bot.eq(false));
    }
    if !author_ids.is_empty() {
        query = query.filter(summary_messages::author_id.eq_any(author_ids));
    }
    let mut messages = query
        .select(summary_messages::content)
        .load_stream::<String>(db)
        .await?
        .par_then_unordered(
            ParParams {
                num_workers: std::thread::available_parallelism()?.get(),
                buf_size: Some(512),
            },
            |value| async move {
                value
                    .expect("Couldn't get message value from summary db")
                    .split_whitespace()
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            },
        );

    let mut chain = Chain::of_order(n_grams);

    while let Some(value) = messages.next().await {
        chain.feed(value);
    }
    Ok(chain)
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
